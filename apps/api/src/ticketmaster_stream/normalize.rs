//! Converting raw Ticketmaster events into this service's response shape,
//! and deriving a stable key for deduplicating events seen across
//! overlapping date windows.

use super::types::{EventResponse, LocationResponse, TmAttraction, TmEvent, VenueResponse};
use crate::spotify::client::normalize_artist_name;
use crate::spotify::top_artists::MatchReason;
use chrono::{DateTime, Local};
use std::collections::HashMap;

pub(crate) fn normalize_event(e: TmEvent) -> EventResponse {
    let dates = e.dates.start.date_time.or_else(|| {
        match (
            e.dates.start.local_date.as_ref(),
            e.dates.start.local_time.as_ref(),
        ) {
            (Some(d), Some(t)) => Some(format!("{d}T{t}")),
            (Some(d), None) => Some(d.clone()),
            _ => None,
        }
    });

    let venue = e
        .embedded
        .as_ref()
        .and_then(|emb| emb.venues.as_ref())
        .and_then(|vs| vs.last().cloned());

    let venue = venue.map(|v| VenueResponse {
        name: v.name,
        location: v.location.map(|loc| LocationResponse {
            latitude: loc.latitude,
            longitude: loc.longitude,
        }),
        city: v.city.and_then(|c| c.name),
    });

    let attractions = e.embedded.and_then(|a| a.attractions);

    let dates_pretty = dates.as_ref().map(|d| {
        DateTime::parse_from_rfc3339(d)
            .map(|dt| dt.with_timezone(&Local).format("%B %d").to_string())
            .unwrap_or_else(|_| d.clone())
    });

    EventResponse {
        id: e.id,
        name: e.name,
        url: e.url,
        venue,
        images: e.images,
        dates,
        dates_pretty,
        classifications: e.classifications,
        attractions,
        price_ranges: e.price_ranges,
        matched_artist: None,
        matched_via: None,
    }
}

/// Sets `matched_artist` (and, for a similar-artist match, `matched_via`) on
/// `event` if any of its Ticketmaster attractions exact-normalize-matches an
/// entry in `matches`. Keys in `matches` must already be exact-normalized
/// (see `spotify::top_artists::get_personalization_matches`). A no-op when
/// the caller has no personalization set (not connected, no session, or
/// fetch failed).
pub(crate) fn apply_personalization(
    event: &mut EventResponse,
    matches: &HashMap<String, MatchReason>,
) {
    if matches.is_empty() {
        return;
    }
    let Some((name, reason)) = event
        .attractions
        .as_ref()
        .and_then(|attractions| find_matching_attraction(attractions, matches))
    else {
        return;
    };

    event.matched_artist = Some(name);
    event.matched_via = match reason {
        MatchReason::Direct => None,
        MatchReason::Similar { seed } => Some(seed),
    };
}

fn find_matching_attraction(
    attractions: &[TmAttraction],
    matches: &HashMap<String, MatchReason>,
) -> Option<(String, MatchReason)> {
    attractions.iter().find_map(|a| {
        let name = a.name.as_ref()?;
        matches
            .get(&normalize_artist_name(name))
            .map(|reason| (name.clone(), reason.clone()))
    })
}

pub(crate) fn dedupe_key(event: &EventResponse) -> String {
    let venue_name = event.venue.as_ref().and_then(|v| v.name.clone());
    let attraction_id = event
        .attractions
        .as_ref()
        .and_then(|xs| xs.first())
        .and_then(|a| a.id.clone());

    format!(
        "{}-{}-{}",
        event.dates.clone().unwrap_or_default(),
        venue_name.unwrap_or_default(),
        attraction_id.unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticketmaster_stream::types::{
        EventEmbedded, TmAttraction, TmDate, TmDateStart, TmVenue,
    };

    fn tm_event(id: &str, start: TmDateStart) -> TmEvent {
        TmEvent {
            id: id.to_string(),
            url: None,
            price_ranges: None,
            name: "Test Event".to_string(),
            images: vec![],
            dates: TmDate { start },
            classifications: None,
            embedded: None,
        }
    }

    #[test]
    fn normalize_event_prefers_date_time() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: Some("2026-08-01T20:00:00Z".to_string()),
                local_date: Some("2026-08-01".to_string()),
                local_time: Some("15:00:00".to_string()),
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates.as_deref(), Some("2026-08-01T20:00:00Z"));
    }

    #[test]
    fn normalize_event_combines_local_date_and_time_when_date_time_missing() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: None,
                local_date: Some("2026-08-01".to_string()),
                local_time: Some("15:00:00".to_string()),
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates.as_deref(), Some("2026-08-01T15:00:00"));
    }

    #[test]
    fn normalize_event_falls_back_to_local_date_only() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: None,
                local_date: Some("2026-08-01".to_string()),
                local_time: None,
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates.as_deref(), Some("2026-08-01"));
    }

    #[test]
    fn normalize_event_has_no_dates_when_all_fields_missing() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: None,
                local_date: None,
                local_time: None,
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates, None);
        assert_eq!(normalized.dates_pretty, None);
    }

    #[test]
    fn normalize_event_uses_last_venue_when_multiple_present() {
        let mut event = tm_event(
            "1",
            TmDateStart {
                date_time: Some("2026-08-01T20:00:00Z".to_string()),
                local_date: None,
                local_time: None,
            },
        );
        event.embedded = Some(EventEmbedded {
            venues: Some(vec![
                TmVenue {
                    name: Some("First Venue".to_string()),
                    location: None,
                    city: None,
                },
                TmVenue {
                    name: Some("Second Venue".to_string()),
                    location: None,
                    city: None,
                },
            ]),
            attractions: None,
        });
        let normalized = normalize_event(event);
        assert_eq!(
            normalized.venue.and_then(|v| v.name),
            Some("Second Venue".to_string())
        );
    }

    #[test]
    fn dedupe_key_is_stable_for_same_date_venue_and_attraction() {
        let event = |id: &str| EventResponse {
            id: id.to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some("The Venue".to_string()),
                location: None,
                city: None,
            }),
            images: vec![],
            dates: Some("2026-08-01T20:00:00Z".to_string()),
            dates_pretty: None,
            classifications: None,
            attractions: Some(vec![TmAttraction {
                name: Some("Artist".to_string()),
                id: Some("artist-1".to_string()),
                classifications: None,
                external_links: None,
                images: None,
            }]),
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
        };

        // Same date/venue/attraction but a different Ticketmaster event id
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
            }),
            images: vec![],
            dates: Some("2026-08-01T20:00:00Z".to_string()),
            dates_pretty: None,
            classifications: None,
            attractions: None,
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
        };
        let mut other = EventResponse {
            venue: Some(VenueResponse {
                name: Some("Venue B".to_string()),
                location: None,
                city: None,
            }),
            ..base.clone()
        };
        other.id = "2".to_string();

        assert_ne!(dedupe_key(&base), dedupe_key(&other));
    }

    fn event_with_attraction(name: &str) -> EventResponse {
        EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: None,
            images: vec![],
            dates: None,
            dates_pretty: None,
            classifications: None,
            attractions: Some(vec![TmAttraction {
                name: Some(name.to_string()),
                id: Some("artist-1".to_string()),
                classifications: None,
                external_links: None,
                images: None,
            }]),
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
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
        let mut event = event_with_attraction("Tyler, The Creator");
        let matches = direct_matches(&["Tyler, The Creator"]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist.as_deref(), Some("Tyler, The Creator"));
        assert_eq!(event.matched_via, None);
    }

    #[test]
    fn apply_personalization_is_case_and_punctuation_insensitive() {
        let mut event = event_with_attraction("Tyler, The Creator");
        let matches = direct_matches(&["tyler the creator"]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist.as_deref(), Some("Tyler, The Creator"));
    }

    #[test]
    fn apply_personalization_leaves_none_when_no_match() {
        let mut event = event_with_attraction("Some Other Artist");
        let matches = direct_matches(&["Tyler, The Creator"]);

        apply_personalization(&mut event, &matches);

        assert_eq!(event.matched_artist, None);
        assert_eq!(event.matched_via, None);
    }

    #[test]
    fn apply_personalization_sets_matched_via_for_similar_artist_match() {
        let mut event = event_with_attraction("Kali Uchis");
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
        let mut event = event_with_attraction("Tyler, The Creator");
        let mut matches = HashMap::new();
        matches.insert(
            normalize_artist_name("Tyler, The Creator"),
            MatchReason::Similar {
                seed: "Kali Uchis".to_string(),
            },
        );
        // A direct match on the same normalized name always wins when
        // building the cache (see `expand_via_similar_artists`); this test
        // just documents that `apply_personalization` renders whichever
        // reason it's given rather than re-deciding precedence itself.
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
        let mut event = event_with_attraction("Tyler, The Creator");

        apply_personalization(&mut event, &HashMap::new());

        assert_eq!(event.matched_artist, None);
    }
}
