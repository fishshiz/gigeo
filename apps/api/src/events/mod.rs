//! The normalized `Event` shape and the source-agnostic logic that operates
//! on it: personalized-discovery matching, event deduplication, and
//! cross-provider stream orchestration (`merge`/`source`/`stream`) —
//! merging Ticketmaster and PredictHQ into one chronologically ordered
//! feed. `ticketmaster_stream`/`predicthq` stay source-specific: raw wire
//! shapes, HTTP fetching, and (for Ticketmaster) the one place that still
//! needs to know Ticketmaster's raw shape (`TmEvent -> EventResponse`
//! mapping). Each satisfies the `EventSource` interface this module
//! defines (`source::EventSource`) via its own `source` submodule.

pub mod types;

mod merge;
pub(crate) mod reconcile;
pub(crate) mod source;
mod stream;

pub(crate) use stream::get_concerts_tm_stream;

use std::collections::HashMap;

use crate::spotify::client::normalize_artist_name;
use crate::spotify::top_artists::MatchReason;
use types::{EventResponse, Performer};

/// Sets `matched_artist` (and, for a similar-artist match, `matched_via`) on
/// `event` if any of its performers exact-normalize-matches an entry in
/// `matches`. Keys in `matches` must already be exact-normalized (see
/// `spotify::top_artists::get_personalization_matches`). A no-op when the
/// caller has no personalization set (not connected, no session, or fetch
/// failed).
pub(crate) fn apply_personalization(
    event: &mut EventResponse,
    matches: &HashMap<String, MatchReason>,
) {
    if matches.is_empty() {
        return;
    }
    let Some((name, reason)) = event
        .performers
        .as_ref()
        .and_then(|performers| find_matching_performer(performers, matches))
    else {
        return;
    };

    event.matched_artist = Some(name);
    event.matched_via = match reason {
        MatchReason::Direct => None,
        MatchReason::Similar { seed } => Some(seed),
    };
}

fn find_matching_performer(
    performers: &[Performer],
    matches: &HashMap<String, MatchReason>,
) -> Option<(String, MatchReason)> {
    performers.iter().find_map(|p| {
        let name = p.name.as_ref()?;
        matches
            .get(&normalize_artist_name(name))
            .map(|reason| (name.clone(), reason.clone()))
    })
}

/// Gates which events feed canonical-artist matching (see
/// `crate::artists::spawn_enrichment`) — Sports/Arts & Theatre/Film events
/// regularly have their own real Apple Music/Spotify catalog entries (a
/// stage musical's cast recording, a team's warmup playlist) that
/// exact-name matching happily "succeeds" against, producing artist
/// enrichment (genres, similar artists, artwork) that makes no sense for
/// what's actually a play or a rodeo. Confirmed live: "The Rocky Horror
/// Show" matched an Apple Music cast-recording page under this exact
/// failure mode.
///
/// Defaults to `true` (attempt the match) whenever there's no primary
/// classification to check — missing metadata shouldn't cost a real music
/// event its enrichment. PredictHQ-sourced events always pass this: their
/// classification's segment is unconditionally `"Music"` (see
/// `predicthq::normalize::build_classifications`), so this only has a real
/// effect on the unscoped Ticketmaster fetch (PredictHQ's own fetch is
/// already `category=concerts`-scoped at the API level).
pub(crate) fn is_music_classified(event: &EventResponse) -> bool {
    let Some(classifications) = &event.classifications else {
        return true;
    };
    let primary = classifications.iter().find(|c| c.primary == Some(true));
    match primary {
        None => true,
        Some(c) => c
            .segment
            .as_ref()
            .is_some_and(|s| s.name.eq_ignore_ascii_case("music")),
    }
}

pub(crate) fn dedupe_key(event: &EventResponse) -> String {
    let venue_name = event.venue.as_ref().and_then(|v| v.name.clone());
    let performer_id = event
        .performers
        .as_ref()
        .and_then(|xs| xs.first())
        .and_then(|p| p.id.clone());

    format!(
        "{}-{}-{}",
        event.dates.clone().unwrap_or_default(),
        venue_name.unwrap_or_default(),
        performer_id.unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{Classification, Segment, Source, VenueResponse};

    fn event_with_performer(name: &str) -> EventResponse {
        EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: None,
            images: vec![],
            dates: None,
            dates_pretty: None,
            local_calendar_day: None,
            classifications: None,
            performers: Some(vec![Performer {
                name: Some(name.to_string()),
                id: Some("artist-1".to_string()),
                classifications: None,
                external_links: None,
                images: None,
                genres: vec![],
            }]),
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        }
    }

    fn direct_matches(names: &[&str]) -> HashMap<String, MatchReason> {
        names
            .iter()
            .map(|n| (normalize_artist_name(n), MatchReason::Direct))
            .collect()
    }

    #[test]
    fn apply_personalization_matches_exact_normalized_name() {
        let mut event = event_with_performer("Tyler, The Creator");
        let matches = direct_matches(&["Tyler, The Creator"]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist.as_deref(), Some("Tyler, The Creator"));
        assert_eq!(event.matched_via, None);
    }

    #[test]
    fn apply_personalization_is_case_and_punctuation_insensitive() {
        let mut event = event_with_performer("Tyler, The Creator");
        let matches = direct_matches(&["tyler the creator"]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist.as_deref(), Some("Tyler, The Creator"));
    }

    #[test]
    fn apply_personalization_leaves_none_when_no_match() {
        let mut event = event_with_performer("Some Other Artist");
        let matches = direct_matches(&["Tyler, The Creator"]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist, None);
        assert_eq!(event.matched_via, None);
    }

    #[test]
    fn apply_personalization_sets_matched_via_for_similar_artist_match() {
        let mut event = event_with_performer("Kali Uchis");
        let matches = HashMap::from([(
            normalize_artist_name("Kali Uchis"),
            MatchReason::Similar {
                seed: "Tyler, The Creator".to_string(),
            },
        )]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist.as_deref(), Some("Kali Uchis"));
        assert_eq!(event.matched_via.as_deref(), Some("Tyler, The Creator"));
    }

    #[test]
    fn apply_personalization_prefers_direct_over_similar_for_the_same_name() {
        let mut event = event_with_performer("Tyler, The Creator");
        let mut matches = HashMap::new();
        matches.insert(
            normalize_artist_name("Tyler, The Creator"),
            MatchReason::Similar {
                seed: "Kali Uchis".to_string(),
            },
        );
        // A direct match on the same normalized name always wins when
        // building the cache (see `spotify::top_artists::expand_via_similar_artists`);
        // this test just documents that `apply_personalization` renders
        // whichever reason it's given rather than re-deciding precedence
        // itself.
        apply_personalization(&mut event, &matches);
        assert_eq!(event.matched_via.as_deref(), Some("Kali Uchis"));

        matches.insert(
            normalize_artist_name("Tyler, The Creator"),
            MatchReason::Direct,
        );
        apply_personalization(&mut event, &matches);
        assert_eq!(event.matched_via, None);
    }

    #[test]
    fn apply_personalization_is_a_noop_with_no_matches() {
        let mut event = event_with_performer("Tyler, The Creator");

        apply_personalization(&mut event, &HashMap::new());

        assert_eq!(event.matched_artist, None);
    }

    #[test]
    fn dedupe_key_is_stable_for_same_date_venue_and_performer() {
        let event = |id: &str| EventResponse {
            id: id.to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some("The Venue".to_string()),
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
            dates: Some("2026-08-01T20:00:00Z".to_string()),
            dates_pretty: None,
            local_calendar_day: None,
            classifications: None,
            performers: Some(vec![Performer {
                name: Some("Artist".to_string()),
                id: Some("artist-1".to_string()),
                classifications: None,
                external_links: None,
                images: None,
                genres: vec![],
            }]),
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        };

        // Same date/venue/performer but a different Ticketmaster event id
        // should still dedupe to the same key — this is the whole point of
        // dedupe_key, since Ticketmaster returns the same show multiple
        // times across overlapping date windows with different ids.
        assert_eq!(dedupe_key(&event("id-a")), dedupe_key(&event("id-b")));
    }

    #[test]
    fn dedupe_key_differs_when_venue_differs() {
        let base = EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some("Venue A".to_string()),
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
            dates: Some("2026-08-01T20:00:00Z".to_string()),
            dates_pretty: None,
            local_calendar_day: None,
            classifications: None,
            performers: None,
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        };
        let mut other = EventResponse {
            venue: Some(VenueResponse {
                name: Some("Venue B".to_string()),
                location: None,
                city: None,
                state: None,
                state_code: None,
                address: None,
                postal_code: None,
                url: None,
                images: vec![],
            }),
            ..base.clone()
        };
        other.id = "2".to_string();

        assert_ne!(dedupe_key(&base), dedupe_key(&other));
    }

    fn classification(primary: bool, segment_name: &str) -> Classification {
        Classification {
            primary: Some(primary),
            segment: Some(Segment {
                id: "id".to_string(),
                name: segment_name.to_string(),
            }),
            genre: None,
            sub_genre: None,
            sub_type: None,
            family: Some(false),
        }
    }

    #[test]
    fn is_music_classified_true_with_no_classifications_at_all() {
        let event = event_with_performer("Artist");
        assert!(is_music_classified(&event));
    }

    #[test]
    fn is_music_classified_true_with_primary_music_segment() {
        let mut event = event_with_performer("Artist");
        event.classifications = Some(vec![classification(true, "Music")]);
        assert!(is_music_classified(&event));
    }

    #[test]
    fn is_music_classified_false_with_primary_non_music_segment() {
        let mut event = event_with_performer("The Rocky Horror Show");
        event.classifications = Some(vec![classification(true, "Arts & Theatre")]);
        assert!(!is_music_classified(&event));
    }

    #[test]
    fn is_music_classified_true_when_no_classification_is_flagged_primary() {
        let mut event = event_with_performer("Artist");
        event.classifications = Some(vec![classification(false, "Sports")]);
        assert!(is_music_classified(&event));
    }

    #[test]
    fn is_music_classified_ignores_non_primary_entries() {
        let mut event = event_with_performer("Artist");
        event.classifications = Some(vec![
            classification(false, "Comedy"),
            classification(true, "Music"),
        ]);
        assert!(is_music_classified(&event));
    }
}
