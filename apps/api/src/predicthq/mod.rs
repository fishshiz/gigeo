//! PredictHQ as a second event source: fetch + normalize
//! (`client`/`normalize`), plus a best-effort Apple Music image/link
//! backfill (`artwork`) for events that pass through
//! `crate::events::reconcile_predicthq_events` unmatched — PredictHQ's own
//! API provides neither field. `source` is this provider's
//! `crate::events::source::EventSource` adapter — the seam
//! `crate::events::merge` merges against. Cross-source dedup and the
//! live-stream wiring itself live in `crate::events` now (`reconcile`,
//! `merge`, `stream`), not here or in `ticketmaster_stream`.

mod artwork;
mod client;
mod normalize;
pub(crate) mod source;

pub(crate) use artwork::backfill_artwork;

use crate::events::types::{EventResponse, Performer};

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

/// Gives `event` a single title-derived performer when it has none at all --
/// the same ~28%-with-no-structured-entity case `performer_search_names`
/// already falls back for, but writing the result onto `event.performers`
/// itself (rather than just returning a search name) so callers that key
/// off `performers` -- the frontend's on-demand `/artists/enrichment` fetch,
/// and `crate::artists::attach_genres` -- have something to find, not just
/// `predicthq::artwork::backfill_artwork` and `crate::artists::worker`
/// (which already call `performer_search_names` directly). `id: None`, same
/// as everywhere else an id-less performer is rendered -- there's no real
/// PredictHQ entity behind this one.
///
/// Only ever safe to call *after* `crate::events::reconcile::reconcile_predicthq_events`
/// has already run against this event (see `events::stream`'s
/// `MergeStep::PredictHqNew` handling, the one caller) -- reconciliation's
/// own performer-name confirmation step relies on `performers` still being
/// `None` at that point to allow a venue+date-only match for a real act
/// PredictHQ just didn't tag, and a synthetic performer here would silently
/// demand an exact name match instead.
pub(crate) fn backfill_title_performer(event: &mut EventResponse) {
    if event.performers.is_none() {
        event.performers = Some(vec![Performer {
            name: Some(event.name.clone()),
            id: None,
            classifications: None,
            external_links: None,
            images: None,
            genres: vec![],
        }]);
    }
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
                state: None,
                state_code: None,
                address: None,
                postal_code: None,
                url: None,
                images: vec![],
            }),
            images: vec![],
            dates: None,
            dates_pretty: None,
            local_calendar_day: None,
            classifications: None,
            performers: performer.map(|name| {
                vec![Performer {
                    name: Some(name.to_string()),
                    id: None,
                    classifications: None,
                    external_links: None,
                    images: None,
                    genres: vec![],
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

    #[test]
    fn backfill_title_performer_fills_in_a_synthetic_performer_when_none() {
        let mut event = phq_event("Craig LaGrassa", None);
        backfill_title_performer(&mut event);

        let performers = event.performers.expect("expected a synthetic performer");
        assert_eq!(performers.len(), 1);
        assert_eq!(performers[0].name.as_deref(), Some("Craig LaGrassa"));
        assert_eq!(performers[0].id, None);
    }

    #[test]
    fn backfill_title_performer_leaves_real_performers_untouched() {
        let mut event = phq_event("The Sauce w/ Jeff Beam", Some("Jeff Beam"));
        backfill_title_performer(&mut event);

        let performers = event.performers.expect("expected the real performer");
        assert_eq!(performers.len(), 1);
        assert_eq!(performers[0].name.as_deref(), Some("Jeff Beam"));
    }
}
