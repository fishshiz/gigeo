//! Backfills images/url for PredictHQ-only events using the same Apple
//! Music catalog artist lookup `crate::artists::worker` uses for canonical
//! artist enrichment — see `crate::apple_music::artwork_cache` for the
//! caching/name-verification rules this relies on. Candidate names come
//! from `super::performer_search_names`, which falls back to the event's
//! own title when there's no structured performer entity at all.
//!
//! Deliberately scoped to *unmatched* PredictHQ events (the `new_events`
//! half of `reconcile_predicthq_events`'s output): a matched event is a
//! clone of its Ticketmaster counterpart, which already has a real photo
//! and ticket URL, so backfilling there would just be wasted API calls.

use futures::{StreamExt, stream};

use crate::apple_music::artwork_cache::{ArtistArtwork, ArtworkCache};
use crate::apple_music::client::AppleMusicClient;
use crate::events::types::{EventResponse, Images};

/// Apple Music artwork URLs are a template (`.../{w}x{h}bb.jpg`) that must
/// be resolved to a concrete size before use — the frontend's own
/// `buildArtworkUrl` does this for the on-demand artist lookup, but
/// `EventResponse.images` is rendered as a plain, ready-to-use URL
/// (`ResponsiveImage` has no notion of Apple's template), so it happens
/// here instead.
const ARTWORK_SIZE_PX: u32 = 1200;

/// Caps how many Apple Music lookups run at once. A cold cache in a dense
/// market can mean a couple dozen unmatched events in one request — firing
/// all of them at once was tried first and reliably tripped Apple's rate
/// limiter (confirmed live: ~30 concurrent lookups produced 32 real 429s
/// in one request), which just pushes the same work through
/// `http_utils::request_with_backoff`'s retry/backoff instead of avoiding
/// it. A small bound still gets most of the concurrency benefit over fully
/// serial lookups without provoking the limiter in the first place.
const MAX_CONCURRENT_LOOKUPS: usize = 4;

/// Populates `images`/`url` on `events` in place from the first performer
/// with a usable Apple Music match, looking up a bounded number of events
/// concurrently (see `MAX_CONCURRENT_LOOKUPS`) rather than either fully
/// serially or all at once.
///
/// Best-effort throughout: an event with no performer name, no confident
/// Apple Music match, or that already has its own image/url, is left
/// exactly as PredictHQ gave it rather than showing something misleading.
pub(crate) async fn backfill_artwork(
    events: &mut [EventResponse],
    am: &AppleMusicClient,
    token: &str,
    cache: &ArtworkCache,
) {
    // Collected as owned `String`s up front (rather than borrowing from
    // `events` inside the stream combinator below) so each lookup future
    // only captures `am`/`token`/`cache` from this function's own scope --
    // borrowing `events` there instead runs into a higher-ranked lifetime
    // error, since `buffered` needs the map closure to work generically
    // over the stream item's lifetime.
    let candidate_names: Vec<Vec<String>> = events
        .iter()
        .map(|event| {
            if !event.images.is_empty() && event.url.is_some() {
                return Vec::new();
            }
            super::performer_search_names(event)
        })
        .collect();

    let results: Vec<Option<ArtistArtwork>> = stream::iter(candidate_names)
        .map(|names| async move {
            for name in &names {
                if let Some(found) = cache.get_or_fetch(am, token, name).await {
                    return Some(found);
                }
            }
            None
        })
        .buffered(MAX_CONCURRENT_LOOKUPS)
        .collect::<Vec<_>>()
        .await;

    for (event, found) in events.iter_mut().zip(results) {
        let Some(found) = found else { continue };

        if event.images.is_empty()
            && let Some(artwork) = &found.artwork
        {
            let url = artwork
                .url
                .replace("{w}", &ARTWORK_SIZE_PX.to_string())
                .replace("{h}", &ARTWORK_SIZE_PX.to_string());
            event.images = vec![Images {
                url,
                width: Some(ARTWORK_SIZE_PX as i32),
                height: Some(ARTWORK_SIZE_PX as i32),
                ratio: None,
                // Signals this is a stand-in artist photo, not a photo of
                // the event itself — same field Ticketmaster-side fallback
                // images already use.
                fallback: Some(true),
            }];
        }

        if event.url.is_none() {
            event.url = found.apple_music_url;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{Performer, Source, VenueResponse};

    fn phq_event(id: &str, performer: Option<&str>) -> EventResponse {
        EventResponse {
            id: id.to_string(),
            name: "Event".to_string(),
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
            dates: Some("2026-08-09T23:00:00Z".to_string()),
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

    // There used to be a test here asserting that an event with no
    // performer data is left untouched with the network path never
    // reached — that stopped being true once candidate names fell back to
    // the event's title (see `super::performer_search_names`), which is
    // covered directly, without a network dependency, by
    // `predicthq::tests::performer_search_names_falls_back_to_event_title_when_no_performers`.

    #[tokio::test]
    async fn skips_events_that_already_have_an_image_and_url() {
        let mut event = phq_event("phq-1", Some("Role Model"));
        event.images = vec![Images {
            url: "https://example.com/real.jpg".to_string(),
            width: None,
            height: None,
            ratio: None,
            fallback: None,
        }];
        event.url = Some("https://tickets.example.com".to_string());
        let mut events = vec![event];

        let am = AppleMusicClient::new(reqwest::Client::new(), "us".to_string());
        let cache = ArtworkCache::new();

        backfill_artwork(&mut events, &am, "unused-token", &cache).await;

        assert_eq!(events[0].images[0].url, "https://example.com/real.jpg");
        assert_eq!(
            events[0].url.as_deref(),
            Some("https://tickets.example.com")
        );
    }
}
