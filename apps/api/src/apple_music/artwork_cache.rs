//! In-memory best-effort cache mapping a normalized artist name to Apple
//! Music catalog artwork/URL, used to backfill images and links for
//! PredictHQ-only events (which carry neither — see `crate::predicthq`).
//!
//! Deliberately process-local with no TTL: artist artwork rarely changes,
//! entries are tiny, and this is a "nice to have" enrichment rather than a
//! source of truth — a stale or absent entry just means one card renders
//! without a photo. Misses are cached too (as `None`), since a meaningful
//! fraction of PredictHQ titles aren't clean artist names (e.g. "The Sauce
//! w/ Jeff Beam") and would otherwise be re-searched, fruitlessly, on every
//! request that includes them.

use std::collections::HashMap;
use tokio::sync::Mutex;

use crate::apple_music::client::{AppleMusicClient, Artwork};
use crate::spotify::client::normalize_artist_name;

#[derive(Debug, Clone)]
pub struct ArtistArtwork {
    pub artwork: Option<Artwork>,
    pub apple_music_url: Option<String>,
}

#[derive(Default)]
pub struct ArtworkCache {
    entries: Mutex<HashMap<String, Option<ArtistArtwork>>>,
}

impl ArtworkCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up `name`'s artwork, using the cache when possible and falling
    /// back to a live Apple Music catalog search on a miss.
    ///
    /// Verifies the top search result's own name actually matches the
    /// query (Apple's search can return an unrelated artist for an
    /// ambiguous or messy term) — an unverified match would silently
    /// attach the wrong photo to an event, which is worse than showing no
    /// photo at all. Returns `None` on no confident match, no artwork, or
    /// any API error; never propagates a failure, since this is a
    /// best-effort enrichment that must not break event streaming.
    pub async fn get_or_fetch(
        &self,
        am: &AppleMusicClient,
        token: &str,
        name: &str,
    ) -> Option<ArtistArtwork> {
        let key = normalize_artist_name(name);
        if key.is_empty() {
            return None;
        }

        if let Some(cached) = self.entries.lock().await.get(&key) {
            return cached.clone();
        }

        let result = Self::fetch(am, token, name, &key).await;
        self.entries.lock().await.insert(key, result.clone());
        result
    }

    async fn fetch(
        am: &AppleMusicClient,
        token: &str,
        name: &str,
        normalized_query: &str,
    ) -> Option<ArtistArtwork> {
        let results = am.search_artists(token, name, 1).await.ok()?;
        let attrs = results.into_iter().next()?.attributes?;

        if normalize_artist_name(&attrs.name) != normalized_query {
            return None;
        }
        if attrs.artwork.is_none() && attrs.url.is_none() {
            return None;
        }

        Some(ArtistArtwork {
            artwork: attrs.artwork,
            apple_music_url: attrs.url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `fetch` requires a live Apple Music API call, so it isn't exercised
    // here — same reasoning as `predicthq::client`'s `#[ignore]`-gated real
    // API tests, kept out of the default `cargo test` run since CI has no
    // network access. Only the pure, network-free short-circuit is
    // covered directly.
    #[tokio::test]
    async fn empty_name_short_circuits_without_touching_the_network_or_cache() {
        let cache = ArtworkCache::new();
        let am = AppleMusicClient::new(reqwest::Client::new(), "us".to_string());
        assert!(cache.get_or_fetch(&am, "token", "").await.is_none());
        assert!(cache.entries.lock().await.is_empty());
    }

    /// Hits the real Apple Music API — not run by default (needs
    /// `APPLE_MUSIC_TEAM_ID`/`APPLE_MUSIC_KEY_ID`/`APPLE_MUSIC_PRIVATE_KEY`
    /// and network access). Run manually with `cargo test -- --ignored` to
    /// confirm the name-match verification and cache both behave correctly
    /// against real search results, not just hand-built fixtures.
    #[tokio::test]
    #[ignore]
    async fn real_lookup_returns_verified_artwork_and_caches_it() {
        use crate::apple_music::auth::{AppleMusicCredentials, DeveloperTokenManager};

        let creds = AppleMusicCredentials::from_env();
        let dev_token = DeveloperTokenManager::new(creds)
            .get_token()
            .await
            .expect("should be able to mint a developer token");

        let am = AppleMusicClient::new(reqwest::Client::new(), "us".to_string());
        let cache = ArtworkCache::new();

        // A clean, unambiguous artist name — expect a confident match with
        // real, resolvable artwork and an apple_music_url.
        let found = cache
            .get_or_fetch(&am, &dev_token, "Role Model")
            .await
            .expect("Role Model should resolve to a real Apple Music artist");
        assert!(found.artwork.is_some(), "expected real artwork");
        assert!(
            found.artwork.as_ref().unwrap().url.contains("{w}"),
            "raw Apple artwork url should still be a template at this layer"
        );
        assert!(found.apple_music_url.is_some());

        // Same name again should be served from cache, not a second
        // network round-trip -- can't observe that directly here, but at
        // minimum the result must be identical.
        let cached = cache.get_or_fetch(&am, &dev_token, "Role Model").await;
        assert_eq!(
            cached.map(|a| a.apple_music_url),
            Some(found.apple_music_url)
        );

        // A messy, non-artist PredictHQ-style title should not produce a
        // false positive match.
        let messy = cache
            .get_or_fetch(&am, &dev_token, "The Sauce w/ Jeff Beam")
            .await;
        assert!(
            messy.is_none(),
            "expected no confident match for a non-artist title, got {messy:?}"
        );
    }
}
