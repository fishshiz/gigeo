//! Fuzzy same-show matching: deciding whether two independently-sourced
//! event listings describe the same real-world show. `same_show` is the one
//! identity check, used two ways — `reconcile_predicthq_events` applies it
//! cross-source (PredictHQ against already-normalized Ticketmaster events;
//! this is the path that caught the confirmed-live Denver bug where
//! Ticketmaster's "La Luz w/ Spacemoth" at "Bluebird Theatre" and
//! PredictHQ's "La Luz" at "Bluebird Theater" showed as two cards --
//! see `normalize_venue_spelling`), `merge_same_source_duplicate` applies
//! it same-source (two differently-titled Ticketmaster listings of the
//! same show).
//!
//! Deliberately a separate concept from `dedupe_key` — that handles the same
//! Ticketmaster event reappearing across overlapping date windows (same
//! source, same identity, literal duplicate, matched on exact fields
//! including title). `same_show` is title-blind and matches on venue +
//! calendar day + performer overlap instead, because none of its callers can
//! assume the two sides agree on a title string.

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
    ticketmaster_events
        .iter()
        .find(|tm| same_show(tm, phq_event))
}

/// Same venue + same calendar day, confirmed by performer-name overlap
/// whenever both sides have performer data. The one identity check this
/// module builds, shared by both directions it's used in: cross-source
/// (`reconcile_predicthq_events`, above) and same-source
/// (`merge_same_source_duplicate`, below) — deliberately title-blind, since
/// neither case can rely on the two sides agreeing on a title string.
pub(crate) fn same_show(a: &EventResponse, b: &EventResponse) -> bool {
    venue_and_date_match(a, b) && performer_confirms_or_absent(a, b)
}

/// Collapses same-source (Ticketmaster) duplicate listings of the same real
/// show within a single date window into one entry in `out` — e.g.
/// Ticketmaster listing the same show twice under different event ids and
/// titles, one carrying a fuller bill than the other. Distinct from
/// `dedupe_key`, which only catches the exact same Ticketmaster event
/// reappearing across overlapping pagination windows (same id/fields,
/// literal duplicate); this catches two different Ticketmaster event ids
/// that `same_show` agrees describe one real show. (The confirmed-live
/// Denver "La Luz" duplicate that motivated `same_show` itself turned out to
/// be cross-source, not same-source -- see `normalize_venue_spelling` --
/// but the same-source case this function handles is a real, distinct
/// failure mode of `dedupe_key` and worth guarding independently.)
///
/// When `event` matches an existing entry, keeps whichever has more
/// performers listed (the fuller bill), tie-broken by longer title — a
/// heuristic for "the more complete listing wins", not a guarantee of
/// picking whichever Ticketmaster considers canonical.
pub(crate) fn merge_same_source_duplicate(out: &mut Vec<EventResponse>, event: EventResponse) {
    match out.iter_mut().find(|existing| same_show(existing, &event)) {
        Some(existing) => {
            if is_richer(&event, existing) {
                *existing = event;
            }
        }
        None => out.push(event),
    }
}

fn is_richer(a: &EventResponse, b: &EventResponse) -> bool {
    let a_count = a.performers.as_ref().map_or(0, |p| p.len());
    let b_count = b.performers.as_ref().map_or(0, |p| p.len());
    if a_count != b_count {
        return a_count > b_count;
    }
    a.name.len() > b.name.len()
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
/// "Thompson's Point" the same way once case/punctuation is stripped (and,
/// per `normalize_venue_spelling`, once British/American word variants like
/// "Theatre"/"Theater" are folded together).
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
    Some(normalize_artist_name(&normalize_venue_spelling(name)))
}

/// Folds away British/American spelling variants different sources use
/// inconsistently for the same physical venue -- confirmed live: Ticketmaster's
/// "Bluebird Theatre" vs. PredictHQ's "Bluebird Theater" for the identical
/// Denver venue. `normalize_artist_name` only lowercases and strips
/// punctuation, so without this the two remain genuinely different strings;
/// this runs first so the exact-name path above can still catch them,
/// rather than falling through to the geo-proximity fallback, which missed
/// this same pair -- their geocoding sits ~0.53mi apart, well outside
/// `VENUE_PROXIMITY_MILES_THRESHOLD`.
///
/// Plain substring replacement, not word-boundary-aware -- deliberately
/// simple, since venue names are proper nouns where an unintended mid-word
/// collision (e.g. "amphitheatre" also folding to "amphitheater") is not a
/// realistic false-positive risk, and is in fact still the correct spelling
/// normalization in that case too.
fn normalize_venue_spelling(name: &str) -> String {
    let mut normalized = name.to_lowercase();
    for (variant, canonical) in VENUE_SPELLING_ALIASES {
        normalized = normalized.replace(variant, canonical);
    }
    normalized
}

const VENUE_SPELLING_ALIASES: &[(&str, &str)] = &[("theatre", "theater"), ("centre", "center")];

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

/// The venue-local calendar day this event falls on, sourced from each
/// provider's own local-time field (see `events::types::EventResponse::local_calendar_day`).
///
/// Deliberately NOT derived from `event.dates` — that's UTC, and for an
/// evening show in any US timezone, splitting a UTC timestamp on `T` yields
/// a day that's one ahead of the actual local day the show is on.
/// Confirmed live against both providers (see the plan/PR this shipped
/// with): a real PredictHQ concert with `start: "2026-11-04T04:00:00Z"` /
/// `start_local: "2026-11-03T20:00:00"`, and real Ticketmaster evening
/// events where `dates.start.dateTime` lands on the next UTC day relative
/// to `dates.start.localDate`.
fn calendar_day(event: &EventResponse) -> Option<&str> {
    event.local_calendar_day.as_deref()
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
                state: None,
                state_code: None,
                address: None,
                postal_code: None,
                url: None,
                images: vec![],
            }),
            images: vec![],
            dates: Some(date.to_string()),
            dates_pretty: None,
            // Mirrors the pre-fix naive `split('T')` behavior so existing
            // fixtures above (which pass a single UTC `dates` string and
            // expect it to double as the calendar day) keep working
            // unchanged; tests that need to exercise a UTC/local-day
            // mismatch override this field explicitly afterward.
            local_calendar_day: Some(date.split('T').next().unwrap_or(date).to_string()),
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
                    genres: vec![],
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
    fn venue_match_treats_theatre_and_theater_spelling_as_the_same_venue() {
        // The confirmed-live Denver bug this regression-tests: Ticketmaster's
        // "La Luz w/ Spacemoth" at "Bluebird Theatre" and PredictHQ's "La
        // Luz" at "Bluebird Theater" -- same real venue, same night, but the
        // spelling difference defeated the exact-name match, and the two
        // providers' own geocoding for it sits ~0.53mi apart (further than
        // `VENUE_PROXIMITY_MILES_THRESHOLD`), so the geo fallback missed it
        // too.
        let tm = vec![with_performers(
            event("tm-la-luz", "Bluebird Theatre", "2026-08-14T02:00:00Z"),
            &["La Luz", "Spacemoth"],
        )];
        let phq = vec![with_performers(
            phq_event("phq-la-luz", "Bluebird Theater", "2026-08-14T02:00:00Z", 41),
            &["La Luz"],
        )];

        let (enrichments, new_events) = reconcile_predicthq_events(&tm, phq);

        assert_eq!(enrichments.len(), 1);
        assert_eq!(enrichments[0].id, "tm-la-luz");
        assert_eq!(enrichments[0].rank, Some(41));
        assert!(new_events.is_empty());
    }

    #[test]
    fn venue_match_treats_centre_and_center_spelling_as_the_same_venue() {
        let tm = vec![event("tm-1", "Bell Centre", "2026-08-07T23:00:00Z")];
        let phq = vec![phq_event(
            "phq-1",
            "Bell Center",
            "2026-08-07T23:00:00Z",
            50,
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
    fn matches_via_local_calendar_day_even_when_utc_dates_disagree() {
        // The confirmed-live bug this regression-tests: Ticketmaster's
        // fallback path (no `dateTime`, bare `localDate` only) doesn't
        // cross midnight, while PredictHQ's `start` (always UTC) does for
        // an evening show -- so the old naive `split('T')` on `dates` saw
        // "2026-10-23" vs "2026-10-24" and never matched, even though both
        // sources agree the show is on the venue-local day of 2026-10-23.
        let mut tm = event("tm-1", "State Theatre", "2026-10-23"); // TM's local-date-only fallback
        tm.local_calendar_day = Some("2026-10-23".to_string());

        let mut phq = phq_event("phq-1", "State Theatre", "2026-10-24T03:00:00Z", 60); // PHQ's UTC start, crossed midnight
        phq.local_calendar_day = Some("2026-10-23".to_string());

        let (enrichments, new_events) = reconcile_predicthq_events(&[tm], vec![phq]);

        assert_eq!(enrichments.len(), 1);
        assert_eq!(enrichments[0].id, "tm-1");
        assert!(new_events.is_empty());
    }

    #[test]
    fn merge_same_source_duplicate_collapses_a_richer_listing_into_the_plainer_one() {
        // The confirmed-live bug: Ticketmaster listed the same Denver show
        // twice under different event ids -- bare "La Luz", and "La Luz w/
        // Spacemoth" with Spacemoth as an added attraction -- same venue,
        // same date, same headliner.
        let mut out = vec![with_performers(
            event("tm-la-luz", "Bluebird Theater", "2026-08-14T02:00:00Z"),
            &["La Luz"],
        )];

        let mut fuller = with_performers(
            event(
                "tm-la-luz-spacemoth",
                "Bluebird Theater",
                "2026-08-14T02:00:00Z",
            ),
            &["La Luz", "Spacemoth"],
        );
        fuller.name = "La Luz w/ Spacemoth".to_string();

        merge_same_source_duplicate(&mut out, fuller);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "La Luz w/ Spacemoth");
        assert_eq!(out[0].performers.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn merge_same_source_duplicate_keeps_the_richer_entry_when_the_plainer_one_arrives_second() {
        let mut fuller = with_performers(
            event(
                "tm-la-luz-spacemoth",
                "Bluebird Theater",
                "2026-08-14T02:00:00Z",
            ),
            &["La Luz", "Spacemoth"],
        );
        fuller.name = "La Luz w/ Spacemoth".to_string();
        let mut out = vec![fuller];

        let plainer = with_performers(
            event("tm-la-luz", "Bluebird Theater", "2026-08-14T02:00:00Z"),
            &["La Luz"],
        );

        merge_same_source_duplicate(&mut out, plainer);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "La Luz w/ Spacemoth");
    }

    #[test]
    fn merge_same_source_duplicate_breaks_an_equal_performer_count_tie_on_title_length() {
        let mut out = vec![with_performers(
            event("tm-1", "Bluebird Theater", "2026-08-14T02:00:00Z"),
            &["La Luz"],
        )];

        let mut fuller_title = with_performers(
            event("tm-2", "Bluebird Theater", "2026-08-14T02:00:00Z"),
            &["La Luz"],
        );
        fuller_title.name = "La Luz (Doors at 7pm)".to_string();

        merge_same_source_duplicate(&mut out, fuller_title);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "La Luz (Doors at 7pm)");
    }

    #[test]
    fn merge_same_source_duplicate_leaves_genuinely_different_shows_separate() {
        let mut out = vec![with_performers(
            event("tm-1", "Bluebird Theater", "2026-08-14T02:00:00Z"),
            &["La Luz"],
        )];

        let other = with_performers(
            event("tm-2", "Bluebird Theater", "2026-08-15T02:00:00Z"),
            &["Another Band"],
        );

        merge_same_source_duplicate(&mut out, other);

        assert_eq!(out.len(), 2);
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
