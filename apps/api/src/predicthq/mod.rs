//! PredictHQ as a second event source: fetch + normalize
//! (`client`/`normalize`), plus a best-effort Apple Music image/link
//! backfill (`artwork`) for events that pass through
//! `crate::events::reconcile_predicthq_events` unmatched — PredictHQ's own
//! API provides neither field. Cross-source dedup and the live-stream
//! wiring live in `ticketmaster_stream` and `crate::events`.

mod artwork;
mod client;
mod normalize;

pub(crate) use artwork::backfill_artwork;
pub(crate) use client::search_concerts;
pub(crate) use normalize::normalize_predicthq_event;

use crate::events::types::EventResponse;

/// Names to search for `event`'s performer(s), in priority order. Falls
/// back to the event's own title when there's no structured performer
/// entity at all (~28% of live PredictHQ events — see
/// `crate::predicthq::normalize`): for a concert listing the title is
/// often just the artist's name, so it's a reasonable last resort even
/// though it's lower-confidence than a real performer entity. Shared by
/// `artwork::backfill_artwork` (image/url backfill on the response) and
/// `crate::artists::worker` (canonical artist enrichment) — both gate
/// whatever this returns behind their own exact-normalized-name
/// verification, so a noisy title (e.g. "The Sauce w/ Jeff Beam") just
/// yields another miss rather than a wrong match.
pub(crate) fn performer_search_names(event: &EventResponse) -> Vec<String> {
    let names: Vec<String> = event
        .performers
        .as_ref()
        .map(|performers| performers.iter().filter_map(|p| p.name.clone()).collect())
        .unwrap_or_default();

    if !names.is_empty() {
        return names;
    }

    vec![event.name.clone()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{Performer, Source, VenueResponse};

    fn phq_event(name: &str, performer: Option<&str>) -> EventResponse {
        EventResponse {
            id: "phq:1".to_string(),
            name: name.to_string(),
            venue: Some(VenueResponse {
                name: Some("Some Venue".to_string()),
                location: None,
                city: None,
            }),
            images: vec![],
            dates: None,
            dates_pretty: None,
            classifications: None,
            performers: performer.map(|name| {
                vec![Performer {
                    name: Some(name.to_string()),
                    id: None,
                    classifications: None,
                    external_links: None,
                    images: None,
                    enrichment: None,
                }]
            }),
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::PredictHq,
            rank: Some(50),
            predicted_attendance: None,
        }
    }

    #[test]
    fn performer_search_names_uses_performer_names_when_present() {
        let event = phq_event("The Sauce w/ Jeff Beam", Some("Jeff Beam"));
        assert_eq!(
            performer_search_names(&event),
            vec!["Jeff Beam".to_string()]
        );
    }

    #[test]
    fn performer_search_names_falls_back_to_event_title_when_no_performers() {
        let event = phq_event("Role Model", None);
        assert_eq!(
            performer_search_names(&event),
            vec!["Role Model".to_string()]
        );
    }
}
