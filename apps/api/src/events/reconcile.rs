//! Cross-source dedup: deciding whether a PredictHQ event is the same show
//! as an already-normalized Ticketmaster event, or genuinely new coverage.
//!
//! Deliberately a separate concept from `dedupe_key` — that handles the same
//! Ticketmaster event reappearing across overlapping date windows (same
//! source, same identity, literal duplicate). This handles two different
//! sources' independent listings of what might be the same real-world show.

use std::collections::HashSet;

use super::types::EventResponse;
use crate::spotify::client::normalize_artist_name;

/// Reconciles PredictHQ events against already-streamed Ticketmaster events.
/// Each PredictHQ event either enriches a matching Ticketmaster event (same
/// venue + calendar date, confirmed by performer name when both sides have
/// one) or becomes its own new card — never both, and Ticketmaster is always
/// preferred as the identity when a match exists (see `Source`'s own docs).
///
/// Returns `(enrichments, new_events)`: `enrichments` are clones of the
/// matched Ticketmaster events with `rank`/`predicted_attendance` set,
/// meant to be re-emitted under their original id as a second-wave update;
/// `new_events` are the unmatched PredictHQ events, unchanged.
pub(crate) fn reconcile_predicthq_events(
    ticketmaster_events: &[EventResponse],
    predicthq_events: Vec<EventResponse>,
) -> (Vec<EventResponse>, Vec<EventResponse>) {
    let mut enrichments = Vec::new();
    let mut new_events = Vec::new();

    for phq_event in predicthq_events {
        match find_matching_ticketmaster_event(ticketmaster_events, &phq_event) {
            Some(matched) => {
                let mut enriched = matched.clone();
                enriched.rank = phq_event.rank;
                enriched.predicted_attendance = phq_event.predicted_attendance;
                enrichments.push(enriched);
            }
            None => new_events.push(phq_event),
        }
    }

    (enrichments, new_events)
}

fn find_matching_ticketmaster_event<'a>(
    ticketmaster_events: &'a [EventResponse],
    phq_event: &EventResponse,
) -> Option<&'a EventResponse> {
    ticketmaster_events.iter().find(|tm| {
        venue_and_date_match(tm, phq_event) && performer_confirms_or_absent(tm, phq_event)
    })
}

fn venue_and_date_match(a: &EventResponse, b: &EventResponse) -> bool {
    let a_day = calendar_day(a);
    let b_day = calendar_day(b);
    if a_day.is_none() || a_day != b_day {
        return false;
    }

    venues_match(a, b)
}

/// Venues match on an exact normalized name, e.g. both sources spelling
/// "Thompson's Point" the same way once case/punctuation is stripped.
///
/// Real sources don't always agree on *which* name to use for the same
/// physical venue, though — observed in Portland, ME testing: PredictHQ
/// attached a show to a named room/stage ("The Rink at Thompson's Point")
/// while Ticketmaster used the parent venue's own name ("Thompson's
/// Point") for the identical show. Neither normalized string matches the
/// other, but they sit ~0.15 miles apart. So we also accept a close geo
/// match — but only when it's confirmed by an actual performer-name
/// overlap (not the "absent = pass" leniency `performer_confirms_or_absent`
/// allows for the exact-name path). Proximity alone is too weak a signal:
/// two genuinely different real venues in this same test market (State
/// Theatre and Aura) sit only ~0.28 miles apart — well within a naive
/// "same complex" radius.
fn venues_match(a: &EventResponse, b: &EventResponse) -> bool {
    let a_venue = venue_key(a);
    let b_venue = venue_key(b);
    if a_venue.is_some() && a_venue == b_venue {
        return true;
    }

    match (venue_coords(a), venue_coords(b)) {
        (Some(a_coords), Some(b_coords)) => {
            distance_miles(a_coords, b_coords) <= VENUE_PROXIMITY_MILES_THRESHOLD
                && performer_names_overlap(a, b)
        }
        _ => false,
    }
}

fn venue_key(event: &EventResponse) -> Option<String> {
    let name = event.venue.as_ref()?.name.as_deref()?;
    Some(normalize_artist_name(name))
}

/// Miles within which two differently-named venues are treated as the same
/// physical complex, provided performer names also confirm the match. Wide
/// enough to cover a named sub-venue (a room, stage, or rink) within a
/// larger complex; tighter than the ~0.28mi gap observed between two
/// genuinely distinct downtown venues in the same test market.
const VENUE_PROXIMITY_MILES_THRESHOLD: f64 = 0.2;

fn venue_coords(event: &EventResponse) -> Option<(f64, f64)> {
    let location = event.venue.as_ref()?.location.as_ref()?;
    let lat: f64 = location.latitude.as_deref()?.parse().ok()?;
    let lng: f64 = location.longitude.as_deref()?.parse().ok()?;
    Some((lat, lng))
}

/// Approximate great-circle distance in miles between two lat/lng points
/// (equirectangular approximation — accurate enough at sub-mile scale
/// without pulling in a geodesy dependency for what's just a "same venue
/// complex?" sanity check).
fn distance_miles(a: (f64, f64), b: (f64, f64)) -> f64 {
    const EARTH_RADIUS_MILES: f64 = 3958.8;
    let (lat1, lng1) = a;
    let (lat2, lng2) = b;
    let avg_lat_rad = ((lat1 + lat2) / 2.0).to_radians();
    let dx = (lng2 - lng1).to_radians() * avg_lat_rad.cos();
    let dy = (lat2 - lat1).to_radians();
    EARTH_RADIUS_MILES * (dx * dx + dy * dy).sqrt()
}

/// Strict performer-name overlap — unlike `performer_confirms_or_absent`,
/// missing performer data on either side does NOT pass. Used to gate the
/// geo-proximity venue fallback, where positive confirmation is required
/// because proximity alone isn't a strong enough signal.
fn performer_names_overlap(a: &EventResponse, b: &EventResponse) -> bool {
    match (performer_names(a), performer_names(b)) {
        (Some(a_names), Some(b_names)) => a_names.intersection(&b_names).next().is_some(),
        _ => false,
    }
}

/// The calendar-day portion of `event.dates` (both sources' dates are UTC
/// ISO 8601 strings, so a plain string split is enough — no timezone math
/// needed to compare "same day").
fn calendar_day(event: &EventResponse) -> Option<&str> {
    event
        .dates
        .as_deref()
        .map(|d| d.split('T').next().unwrap_or(d))
}

/// When both sides have structured performer data, require at least one
/// name to match (exact-normalized, same discipline as personalization
/// matching) before confirming a venue+date match — guards against, e.g., a
/// matinee and an evening show at the same venue on the same day getting
/// merged. When either side lacks performer data, venue+date alone is
/// already the strongest signal available, so it stands on its own.
fn performer_confirms_or_absent(tm: &EventResponse, phq: &EventResponse) -> bool {
    let tm_names = performer_names(tm);
    let phq_names = performer_names(phq);

    match (tm_names, phq_names) {
        (Some(tm_names), Some(phq_names)) => tm_names.intersection(&phq_names).next().is_some(),
        _ => true,
    }
}

fn performer_names(event: &EventResponse) -> Option<HashSet<String>> {
    let performers = event.performers.as_ref()?;
    let names: HashSet<String> = performers
        .iter()
        .filter_map(|p| p.name.as_deref())
        .map(normalize_artist_name)
        .collect();
    (!names.is_empty()).then_some(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{Performer, Source, VenueResponse};

    fn event(id: &str, venue: &str, date: &str) -> EventResponse {
        EventResponse {
            id: id.to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some(venue.to_string()),
                location: None,
                city: None,
            }),
            images: vec![],
            dates: Some(date.to_string()),
            dates_pretty: None,
            classifications: None,
            performers: None,
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        }
    }

    fn with_location(mut e: EventResponse, lat: &str, lng: &str) -> EventResponse {
        e.venue.as_mut().unwrap().location = Some(crate::events::types::LocationResponse {
            latitude: Some(lat.to_string()),
            longitude: Some(lng.to_string()),
        });
        e
    }

    fn with_performers(mut e: EventResponse, names: &[&str]) -> EventResponse {
        e.performers = Some(
            names
                .iter()
                .map(|n| Performer {
                    name: Some(n.to_string()),
                    id: None,
                    classifications: None,
                    external_links: None,
                    images: None,
                    enrichment: None,
                })
                .collect(),
        );
        e
    }

    fn phq_event(id: &str, venue: &str, date: &str, rank: u8) -> EventResponse {
        let mut e = event(id, venue, date);
        e.source = Source::PredictHq;
        e.rank = Some(rank);
        e.predicted_attendance = Some(500);
        e
    }

    #[test]
    fn matches_on_venue_and_date_alone_when_neither_side_has_performers() {
        let tm = vec![event("tm-1", "State Theatre", "2026-08-09T23:00:00Z")];
        let phq = vec![phq_event(
            "phq-1",
            "State Theatre",
            "2026-08-09T23:00:00Z",
            60,
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert_eq!(enrichments.len(), 1);
        assert_eq!(enrichments[0].id, "tm-1");
        assert_eq!(enrichments[0].rank, Some(60));
        assert_eq!(enrichments[0].predicted_attendance, Some(500));
        assert_eq!(enrichments[0].source, Source::Ticketmaster);
        assert!(new_events.is_empty());
    }

    #[test]
    fn does_not_match_when_venue_differs() {
        let tm = vec![event("tm-1", "State Theatre", "2026-08-09T23:00:00Z")];
        let phq = vec![phq_event(
            "phq-1",
            "Merrill Auditorium",
            "2026-08-09T23:00:00Z",
            60,
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
    }

    #[test]
    fn does_not_match_when_date_differs() {
        let tm = vec![event("tm-1", "State Theatre", "2026-08-09T23:00:00Z")];
        let phq = vec![phq_event(
            "phq-1",
            "State Theatre",
            "2026-08-10T23:00:00Z",
            60,
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
    }

    #[test]
    fn requires_performer_confirmation_when_both_sides_have_performers() {
        let tm = vec![with_performers(
            event("tm-1", "Thompson's Point", "2026-08-08T23:00:00Z"),
            &["Guster"],
        )];
        // Same venue+date, but a different performer entirely on both
        // sides — e.g. a matinee vs. an evening show sharing a venue/day.
        let phq = vec![with_performers(
            phq_event("phq-1", "Thompson's Point", "2026-08-08T23:00:00Z", 60),
            &["Some Other Act"],
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
    }

    #[test]
    fn confirms_match_when_performer_names_overlap() {
        let tm = vec![with_performers(
            event("tm-1", "Thompson's Point", "2026-08-08T23:00:00Z"),
            &["Guster"],
        )];
        let phq = vec![with_performers(
            phq_event("phq-1", "Thompson's Point", "2026-08-08T23:00:00Z", 60),
            &["Iron & Wine", "Guster"],
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert_eq!(enrichments.len(), 1);
        assert!(new_events.is_empty());
    }

    #[test]
    fn matches_when_only_predicthq_side_lacks_performers() {
        // The Barr Brothers case: a real act with no structured PredictHQ
        // entity — venue+date alone must still be sufficient.
        let tm = vec![with_performers(
            event("tm-1", "Thompson's Point", "2026-08-08T23:00:00Z"),
            &["The Barr Brothers"],
        )];
        let phq = vec![phq_event(
            "phq-1",
            "Thompson's Point",
            "2026-08-08T23:00:00Z",
            39,
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert_eq!(enrichments.len(), 1);
        assert!(new_events.is_empty());
    }

    #[test]
    fn unmatched_predicthq_events_pass_through_unchanged_as_new_events() {
        let tm = vec![event("tm-1", "State Theatre", "2026-08-09T23:00:00Z")];
        let phq = vec![phq_event("phq-1", "SPACE", "2026-08-06T23:00:00Z", 37)];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
        assert_eq!(new_events[0].id, "phq-1");
        assert_eq!(new_events[0].source, Source::PredictHq);
    }

    #[test]
    fn venue_match_is_case_and_punctuation_insensitive() {
        let tm = vec![event(
            "tm-1",
            "One Longfellow Square",
            "2026-08-07T23:00:00Z",
        )];
        let phq = vec![phq_event(
            "phq-1",
            "one longfellow square",
            "2026-08-07T23:00:00Z",
            36,
        )];

        let (enrichments, _) = reconcile_predicthq_events(&tm, phq);

        assert_eq!(enrichments.len(), 1);
    }

    #[test]
    fn matches_via_geo_proximity_when_venue_names_differ_but_performer_confirms() {
        // The real Portland, ME case: Ticketmaster's "Role Model Presents:
        // Chuck Comes Home" at "Thompson's Point" vs. PredictHQ's "Role
        // Model" at "The Rink at Thompson's Point" -- same show, same day,
        // ~0.15 miles apart, names don't normalize to the same string.
        let tm = vec![with_location(
            with_performers(
                event("tm-1", "Thompson's Point", "2026-08-07T23:00:00Z"),
                &["Role Model"],
            ),
            "43.648693",
            "-70.289493",
        )];
        let phq = vec![with_location(
            with_performers(
                phq_event(
                    "phq-1",
                    "The Rink at Thompson's Point",
                    "2026-08-07T23:00:00Z",
                    60,
                ),
                &["Role Model"],
            ),
            "43.649943",
            "-70.291827",
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert_eq!(enrichments.len(), 1);
        assert_eq!(enrichments[0].id, "tm-1");
        assert!(new_events.is_empty());
    }

    #[test]
    fn geo_proximity_alone_without_performer_confirmation_does_not_match() {
        // Two genuinely different venues in the same test market sitting
        // close together (State Theatre / Aura, ~0.28mi apart) must not
        // merge just because they're near each other and neither side
        // happens to carry performer data.
        let tm = vec![with_location(
            event("tm-1", "State Theatre", "2026-08-09T23:00:00Z"),
            "43.65397160",
            "-70.26329240",
        )];
        let phq = vec![with_location(
            phq_event("phq-1", "Aura", "2026-08-09T23:00:00Z", 50),
            "43.65655100",
            "-70.25894300",
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
    }

    #[test]
    fn geo_proximity_with_conflicting_performers_does_not_match() {
        let tm = vec![with_location(
            with_performers(
                event("tm-1", "Thompson's Point", "2026-08-07T23:00:00Z"),
                &["Role Model"],
            ),
            "43.648693",
            "-70.289493",
        )];
        let phq = vec![with_location(
            with_performers(
                phq_event(
                    "phq-1",
                    "The Rink at Thompson's Point",
                    "2026-08-07T23:00:00Z",
                    60,
                ),
                &["Some Other Act"],
            ),
            "43.649943",
            "-70.291827",
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
    }

    #[test]
    fn no_venue_on_either_side_never_matches() {
        let mut tm_event = event("tm-1", "placeholder", "2026-08-09T23:00:00Z");
        tm_event.venue = None;
        let mut phq_evt = phq_event("phq-1", "placeholder", "2026-08-09T23:00:00Z", 60);
        phq_evt.venue = None;

        let (enrichments, new_events) = reconcile_predicthq_events(&[tm_event], vec![phq_evt]);

        assert!(enrichments.is_empty());
        assert_eq!(new_events.len(), 1);
    }
}
