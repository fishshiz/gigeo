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
    let a_venue = venue_key(a);
    let b_venue = venue_key(b);
    if a_venue.is_none() || a_venue != b_venue {
        return false;
    }

    let a_day = calendar_day(a);
    let b_day = calendar_day(b);
    a_day.is_some() && a_day == b_day
}

fn venue_key(event: &EventResponse) -> Option<String> {
    let name = event.venue.as_ref()?.name.as_deref()?;
    Some(normalize_artist_name(name))
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
