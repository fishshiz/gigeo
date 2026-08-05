use std::collections::HashMap;

use chrono::{Duration, Utc};
use futures::{StreamExt, stream};

use super::db::{self, MatchedArtist};
use super::{ArtistCandidate, SimilarArtist};
use crate::apple_music::client::ArtistAttributes;
use crate::spotify::client::{Artist as SpotifyArtist, normalize_artist_name};
use crate::state::AppState;

/// Apple Music's own artwork template resolved to a real photo size for an
/// artist's primary image (matches `predicthq::artwork`'s existing
/// constant — kept local since this module doesn't otherwise depend on
/// that one, and the two are free to diverge).
const ARTWORK_SIZE_PX: u32 = 1200;
/// Similar-artist thumbnails render much smaller, so there's no reason to
/// pull the same full-size image for a list of a dozen related artists.
const SIMILAR_ARTWORK_SIZE_PX: u32 = 300;

/// Caps concurrent Apple Music/Spotify lookups for the same reason
/// `predicthq::artwork::MAX_CONCURRENT_LOOKUPS` does — firing every lookup
/// from a dense request at once reliably trips Apple's rate limiter.
const MAX_CONCURRENT_ENRICHMENTS: usize = 4;

/// How long to wait before retrying a name that previously found no
/// confident match anywhere. Not zero and not forever: catalogs do change,
/// but most "misses" are noisy PredictHQ titles that will never resolve,
/// so this isn't retried aggressively.
const UNMATCHED_RETRY_COOLDOWN: Duration = Duration::days(7);
/// How long a matched artist's dynamic fields (genres, similar artists) are
/// trusted before being re-fetched. Identity and artwork have no
/// equivalent TTL — see ADR-0001.
const DYNAMIC_FIELD_TTL: Duration = Duration::days(30);

/// Fires off best-effort background enrichment for `candidates` without
/// blocking the caller — see module docs on why this, and not a batch job
/// over stored events, is what triggers enrichment at all.
pub(crate) fn spawn_enrichment(state: &AppState, candidates: Vec<ArtistCandidate>) {
    if candidates.is_empty() {
        return;
    }

    let state = state.clone();
    tokio::spawn(async move {
        let candidates = dedupe_candidates(candidates);
        stream::iter(candidates)
            .for_each_concurrent(MAX_CONCURRENT_ENRICHMENTS, |candidate| {
                let state = state.clone();
                async move { resolve_one(&state, candidate).await }
            })
            .await;
    });
}

/// Dedupes by normalized name so a request with dozens of events by the
/// same artist doesn't queue the same lookup dozens of times, keeping
/// whichever candidate carries a Ticketmaster attraction id (a stronger
/// signal than a bare name) when more than one is seen.
fn dedupe_candidates(candidates: Vec<ArtistCandidate>) -> Vec<ArtistCandidate> {
    let mut by_key: HashMap<String, ArtistCandidate> = HashMap::new();
    for candidate in candidates {
        let key = normalize_artist_name(&candidate.name);
        if key.is_empty() {
            continue;
        }
        by_key
            .entry(key)
            .and_modify(|existing| {
                if existing.ticketmaster_attraction_id.is_none() {
                    existing.ticketmaster_attraction_id =
                        candidate.ticketmaster_attraction_id.clone();
                }
            })
            .or_insert(candidate);
    }
    by_key.into_values().collect()
}

async fn resolve_one(state: &AppState, candidate: ArtistCandidate) {
    let normalized = normalize_artist_name(&candidate.name);
    if normalized.is_empty() {
        return;
    }

    let existing = match db::get_artist(&state.db.pool, &normalized).await {
        Ok(row) => row,
        Err(err) => {
            tracing::warn!(error = %err, name = %candidate.name, "artist enrichment: failed to read existing row, skipping");
            return;
        }
    };

    if let Some(row) = &existing {
        if row.matched_via.is_some() {
            let fresh = row
                .enrichment_refreshed_at
                .is_some_and(|t| Utc::now() - t < DYNAMIC_FIELD_TTL);
            if !fresh {
                refresh_dynamic_fields(state, &normalized, row).await;
            }
            return;
        }

        if Utc::now() - row.last_attempted_at < UNMATCHED_RETRY_COOLDOWN {
            return;
        }
        // Cooldown elapsed on a previous no-match — fall through and retry.
    }

    resolve_fresh(state, &candidate, &normalized).await;
}

async fn resolve_fresh(state: &AppState, candidate: &ArtistCandidate, normalized: &str) {
    if let Some((apple_music_id, attrs)) =
        try_apple_music_match(state, &candidate.name, normalized).await
    {
        let similar = fetch_similar_artists(state, &apple_music_id).await;
        let artwork_url = attrs
            .artwork
            .as_ref()
            .map(|a| resolve_artwork_url(&a.url, ARTWORK_SIZE_PX));
        let artwork_bg_color = attrs.artwork.as_ref().and_then(|a| a.bg_color.clone());

        let result = db::upsert_matched(
            &state.db.pool,
            MatchedArtist {
                normalized_name: normalized,
                display_name: &attrs.name,
                apple_music_id: Some(&apple_music_id),
                spotify_id: None,
                ticketmaster_attraction_id: candidate.ticketmaster_attraction_id.as_deref(),
                matched_via: "apple_music",
                artwork_url: artwork_url.as_deref(),
                artwork_bg_color: artwork_bg_color.as_deref(),
                apple_music_url: attrs.url.as_deref(),
                spotify_url: None,
                genres: &attrs.genre_names,
                similar_artists: &similar,
            },
        )
        .await;

        if let Err(err) = result {
            tracing::warn!(error = %err, name = %candidate.name, "artist enrichment: failed to persist Apple Music match");
        }
        return;
    }

    if let Some(artist) = try_spotify_match(state, &candidate.name, normalized).await {
        let artwork_url = artist.images.first().map(|i| i.url.as_str());
        let result = db::upsert_matched(
            &state.db.pool,
            MatchedArtist {
                normalized_name: normalized,
                display_name: &artist.name,
                apple_music_id: None,
                spotify_id: Some(&artist.id),
                ticketmaster_attraction_id: candidate.ticketmaster_attraction_id.as_deref(),
                matched_via: "spotify",
                artwork_url,
                artwork_bg_color: None,
                apple_music_url: None,
                spotify_url: artist.external_urls.spotify.as_deref(),
                genres: &artist.genres,
                // Only Apple Music has a live similar-artists view (see
                // module docs) — an artist Apple couldn't verify simply
                // has none.
                similar_artists: &[],
            },
        )
        .await;

        if let Err(err) = result {
            tracing::warn!(error = %err, name = %candidate.name, "artist enrichment: failed to persist Spotify match");
        }
        return;
    }

    let result = db::upsert_tombstone(
        &state.db.pool,
        normalized,
        &candidate.name,
        candidate.ticketmaster_attraction_id.as_deref(),
    )
    .await;
    if let Err(err) = result {
        tracing::warn!(error = %err, name = %candidate.name, "artist enrichment: failed to persist no-match tombstone");
    }
}

/// Refreshes genres/similar-artists for an already-matched row using its
/// stored provider id (a direct lookup, not a re-search — identity is
/// already trusted, so there's no need to re-verify it).
async fn refresh_dynamic_fields(state: &AppState, normalized_name: &str, row: &db::ArtistRow) {
    match row.matched_via.as_deref() {
        Some("apple_music") => {
            let (Some(am), Some(dev_mgr)) = (&state.apple_music_client, &state.apple_dev_token)
            else {
                return;
            };
            let Some(apple_music_id) = &row.apple_music_id else {
                return;
            };
            let Ok(token) = dev_mgr.get_token().await else {
                return;
            };
            let Ok(artist) = am.get_artist(&token, apple_music_id).await else {
                let _ = db::touch_last_attempted(&state.db.pool, normalized_name).await;
                return;
            };
            let similar = fetch_similar_artists(state, apple_music_id).await;
            let genres = artist.attributes.map(|a| a.genre_names).unwrap_or_default();

            if let Err(err) =
                db::refresh_dynamic(&state.db.pool, normalized_name, &genres, &similar).await
            {
                tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to refresh Apple Music dynamic fields");
            }
        }
        Some("spotify") => {
            let Some(spotify_id) = &row.spotify_id else {
                return;
            };
            let Ok(token) = state.cc_manager.get_token().await else {
                return;
            };
            let Ok(artist) = state.spotify.get_artist(&token.access_token, spotify_id).await
            else {
                let _ = db::touch_last_attempted(&state.db.pool, normalized_name).await;
                return;
            };

            if let Err(err) =
                db::refresh_dynamic(&state.db.pool, normalized_name, &artist.genres, &[]).await
            {
                tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to refresh Spotify dynamic fields");
            }
        }
        _ => {}
    }
}

/// Verified Apple Music catalog search: returns a match only when the top
/// result's own name normalizes to the same string as the query, since
/// Apple's search can return an unrelated artist for an ambiguous or messy
/// term — same rule `apple_music::artwork_cache` already applies.
async fn try_apple_music_match(
    state: &AppState,
    name: &str,
    normalized: &str,
) -> Option<(String, ArtistAttributes)> {
    let am = state.apple_music_client.as_ref()?;
    let dev_mgr = state.apple_dev_token.as_ref()?;
    let token = dev_mgr.get_token().await.ok()?;

    let candidate = am.search_artists(&token, name, 1).await.ok()?.into_iter().next()?;
    let attrs = candidate.attributes?;
    if normalize_artist_name(&attrs.name) != normalized {
        return None;
    }
    Some((candidate.id, attrs))
}

/// Verified Spotify catalog search, under the same name-match rule as
/// `try_apple_music_match`. Only called when Apple Music had no verified
/// match for this name.
async fn try_spotify_match(state: &AppState, name: &str, normalized: &str) -> Option<SpotifyArtist> {
    let token = state.cc_manager.get_token().await.ok()?;
    let candidate = state
        .spotify
        .search_artist(&token.access_token, name, 1)
        .await
        .ok()?
        .into_iter()
        .next()?;
    (normalize_artist_name(&candidate.name) == normalized).then_some(candidate)
}

async fn fetch_similar_artists(state: &AppState, apple_music_id: &str) -> Vec<SimilarArtist> {
    let (Some(am), Some(dev_mgr)) = (&state.apple_music_client, &state.apple_dev_token) else {
        return Vec::new();
    };
    let Ok(token) = dev_mgr.get_token().await else {
        return Vec::new();
    };

    am.get_similar_artists(&token, apple_music_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|resource| {
            let attrs = resource.attributes?;
            Some(SimilarArtist {
                name: attrs.name,
                apple_music_id: resource.id,
                apple_music_url: attrs.url,
                artwork_url: attrs
                    .artwork
                    .map(|a| resolve_artwork_url(&a.url, SIMILAR_ARTWORK_SIZE_PX)),
            })
        })
        .collect()
}

fn resolve_artwork_url(template: &str, size_px: u32) -> String {
    template
        .replace("{w}", &size_px.to_string())
        .replace("{h}", &size_px.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_candidates_collapses_repeats_of_the_same_artist() {
        let candidates = vec![
            ArtistCandidate {
                name: "Role Model".into(),
                ticketmaster_attraction_id: None,
            },
            ArtistCandidate {
                name: "role model".into(),
                ticketmaster_attraction_id: None,
            },
        ];

        assert_eq!(dedupe_candidates(candidates).len(), 1);
    }

    #[test]
    fn dedupe_candidates_backfills_a_missing_attraction_id_from_a_later_duplicate() {
        let candidates = vec![
            ArtistCandidate {
                name: "Role Model".into(),
                ticketmaster_attraction_id: None,
            },
            ArtistCandidate {
                name: "Role Model".into(),
                ticketmaster_attraction_id: Some("K123".into()),
            },
        ];

        let deduped = dedupe_candidates(candidates);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].ticketmaster_attraction_id.as_deref(), Some("K123"));
    }

    #[test]
    fn dedupe_candidates_keeps_the_first_attraction_id_over_a_later_none() {
        let candidates = vec![
            ArtistCandidate {
                name: "Role Model".into(),
                ticketmaster_attraction_id: Some("K123".into()),
            },
            ArtistCandidate {
                name: "Role Model".into(),
                ticketmaster_attraction_id: None,
            },
        ];

        let deduped = dedupe_candidates(candidates);
        assert_eq!(deduped[0].ticketmaster_attraction_id.as_deref(), Some("K123"));
    }

    #[test]
    fn dedupe_candidates_drops_names_that_normalize_to_empty() {
        let candidates = vec![ArtistCandidate {
            name: "!!!".into(),
            ticketmaster_attraction_id: None,
        }];

        assert!(dedupe_candidates(candidates).is_empty());
    }

    #[test]
    fn resolve_artwork_url_replaces_both_size_placeholders() {
        let url = resolve_artwork_url("https://example.com/{w}x{h}bb.jpg", 300);
        assert_eq!(url, "https://example.com/300x300bb.jpg");
    }
}
