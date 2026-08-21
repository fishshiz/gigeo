use std::collections::HashMap;

use chrono::{Duration, Utc};
use futures::{StreamExt, stream};

use super::cleaning;
use super::db::{self, MatchedArtist, SimilarArtistsStatus};
use super::{ArtistCandidate, SimilarArtist};
use crate::apple_music::client::ArtistAttributes;
use crate::auth::ClientCredentialsManager;
use crate::spotify::client::{
    Artist as SpotifyArtist, SimplifiedAlbum, SpotifyClient, normalize_artist_name,
};
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
/// How long a "latest release" check is trusted before
/// `backfill_release_info` re-fetches it — deliberately much shorter than
/// `DYNAMIC_FIELD_TTL`: unlike genres/similar-artists, whether an artist
/// has released something in the last 30 days changes on its own
/// timescale, so this needs a tighter cadence to avoid the recency flag
/// going stale for most of the window it's supposed to cover.
const RELEASE_CHECK_TTL: Duration = Duration::days(3);

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

pub(super) async fn resolve_one(state: &AppState, candidate: ArtistCandidate) {
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
    // Both providers are always attempted now, concurrently -- independent
    // read-only lookups, so there's no reason to serialize them. Apple
    // Music still wins identity/display fields when it has a match
    // (display_name, artwork, genres, similar_artists -- see module docs /
    // ADR-0001 on why); a Spotify match found alongside it only ever
    // contributes spotify_id/spotify_url on top.
    let (apple_match, spotify_match) = tokio::join!(
        try_apple_music_match(state, &candidate.name),
        try_spotify_match(&state.spotify, &state.cc_manager, &candidate.name),
    );

    if let Some((apple_music_id, attrs)) = apple_match {
        // Similar-artists is deliberately NOT fetched at first-match time --
        // an extra Apple Music call for every newly-seen candidate, most of
        // which nobody ever opens event details for. Backfilled lazily by
        // `backfill_similar_artists`, called only from the on-demand
        // `/artists/enrichment` endpoint when a selected event's performer
        // actually needs it.
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
                spotify_id: spotify_match.as_ref().map(|a| a.id.as_str()),
                ticketmaster_attraction_id: candidate.ticketmaster_attraction_id.as_deref(),
                matched_via: "apple_music",
                artwork_url: artwork_url.as_deref(),
                artwork_bg_color: artwork_bg_color.as_deref(),
                apple_music_url: attrs.url.as_deref(),
                spotify_url: spotify_match
                    .as_ref()
                    .and_then(|a| a.external_urls.spotify.as_deref()),
                genres: &attrs.genre_names,
                similar_artists: &[],
                // Only present when the concurrent Spotify lookup also
                // found a confident match -- see `MatchedArtist::popularity`.
                popularity: spotify_match.as_ref().map(|a| a.popularity as i16),
            },
        )
        .await;

        if let Err(err) = result {
            tracing::warn!(error = %err, name = %candidate.name, "artist enrichment: failed to persist Apple Music match");
        }
        return;
    }

    if let Some(artist) = spotify_match {
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
                popularity: Some(artist.popularity as i16),
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
            // Popularity only ever comes from Spotify -- an Apple-matched
            // row only has one to refresh if the concurrent lookup at
            // first-match time also found a Spotify id (see
            // `resolve_fresh`). `None` here is "nothing new to report", not
            // "clear it" -- `db::refresh_dynamic` coalesces accordingly.
            let popularity = fetch_spotify_popularity(state, row.spotify_id.as_deref()).await;

            if let Err(err) = db::refresh_dynamic(
                &state.db.pool,
                normalized_name,
                &genres,
                &similar,
                popularity,
            )
            .await
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
            let Ok(artist) = state
                .spotify
                .get_artist(&token.access_token, spotify_id)
                .await
            else {
                let _ = db::touch_last_attempted(&state.db.pool, normalized_name).await;
                return;
            };

            if let Err(err) = db::refresh_dynamic(
                &state.db.pool,
                normalized_name,
                &artist.genres,
                &[],
                Some(artist.popularity as i16),
            )
            .await
            {
                tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to refresh Spotify dynamic fields");
            }
        }
        _ => {}
    }
}

/// Best-effort Spotify popularity lookup by id, for the Apple-matched
/// branch of `refresh_dynamic_fields` -- `None` for no id, an expired/
/// unobtainable client-credentials token, or a failed Spotify call, same
/// "just don't update it" handling as everywhere else in this best-effort
/// pipeline.
async fn fetch_spotify_popularity(state: &AppState, spotify_id: Option<&str>) -> Option<i16> {
    let spotify_id = spotify_id?;
    let token = state.cc_manager.get_token().await.ok()?;
    let artist = state
        .spotify
        .get_artist(&token.access_token, spotify_id)
        .await
        .ok()?;
    Some(artist.popularity as i16)
}

/// Fetches and persists similar-artists for `normalized_name` if it's
/// Apple-Music-matched and doesn't have any yet -- the on-demand
/// counterpart to the fetch `resolve_fresh` deliberately skips at
/// first-match time (see that function's comment). A no-op for a
/// Spotify-only match (similar artists only ever come from Apple Music,
/// same as `resolve_fresh`), an unmatched row, or one that already has
/// similar artists (from an earlier on-demand call, or the periodic
/// dynamic-field refresh in `refresh_dynamic_fields`, which still fetches
/// both together on its own much rarer 30-day cadence).
///
/// Doesn't retry a genuinely-empty result (a real artist with no similar-
/// artists data, or a transient fetch failure) on every subsequent detail-
/// view open -- best-effort, same as everywhere else in this module.
pub(super) async fn backfill_similar_artists(state: &AppState, normalized_name: &str) {
    let existing_similar = match db::get_similar_artists(&state.db.pool, normalized_name).await {
        Ok(existing) => existing,
        Err(err) => {
            tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to read similar-artists status, skipping backfill");
            return;
        }
    };
    let Some(SimilarArtistsStatus {
        apple_music_id: Some(apple_music_id),
        has_similar_artists: false,
    }) = existing_similar
    else {
        return;
    };

    let similar = fetch_similar_artists(state, &apple_music_id).await;
    if similar.is_empty() {
        return;
    }

    if let Err(err) = db::attach_similar_artists(&state.db.pool, normalized_name, &similar).await {
        tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to backfill similar artists");
    }
}

/// Fetches and persists `normalized_name`'s latest release, if its
/// `release_checked_at` is missing or past `RELEASE_CHECK_TTL` -- the
/// on-demand counterpart to the fetch `resolve_fresh` never does at
/// first-match time (same "most newly-seen artists are never opened"
/// reasoning as `backfill_similar_artists`). Works off `spotify_id`
/// directly rather than `matched_via`: an Apple-matched row can still
/// carry one from the concurrent Spotify lookup, and popularity/release
/// data only ever comes from Spotify regardless of which provider won
/// identity — see `db::get_release_check_status`.
///
/// Persists a "checked, nothing found" result (`release: None`) on a
/// successful-but-empty fetch, not just a successful one with a hit —
/// unlike `backfill_similar_artists`, this has its own short TTL to
/// re-check on, so there's no need to leave `release_checked_at` unset to
/// force a retry on every subsequent open.
pub(super) async fn backfill_release_info(state: &AppState, normalized_name: &str) {
    let status = match db::get_release_check_status(&state.db.pool, normalized_name).await {
        Ok(Some(status)) => status,
        Ok(None) => return,
        Err(err) => {
            tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to read release-check status, skipping backfill");
            return;
        }
    };

    let Some(spotify_id) = status.spotify_id else {
        return;
    };
    let fresh = status
        .release_checked_at
        .is_some_and(|t| Utc::now() - t < RELEASE_CHECK_TTL);
    if fresh {
        return;
    }

    let Ok(token) = state.cc_manager.get_token().await else {
        return;
    };
    // A failed albums fetch (rate limit, transient error) leaves
    // `release_checked_at` untouched entirely, rather than persisting a
    // false "nothing found" — retried on the very next on-demand call
    // instead of waiting out the full TTL. The artist-popularity refetch
    // below piggybacks on this same on-demand call (a cheap second
    // request on a token we already have) mainly to backfill `popularity`
    // for rows matched before that column existed — see the module-level
    // note on `resolve_fresh`/`refresh_dynamic_fields` for the two other
    // paths that also keep it fresh going forward. Its own failure is
    // independently best-effort and doesn't block the release check.
    let (albums, popularity) = tokio::join!(
        state.spotify.get_artist_albums(&token.access_token, &spotify_id),
        fetch_spotify_popularity(state, Some(&spotify_id)),
    );
    let Ok(albums) = albums else {
        return;
    };

    let latest = albums
        .iter()
        .filter_map(|album| parse_release_date(album).map(|date| (date, album)))
        .max_by_key(|(date, _)| *date);

    let release = latest.map(|(release_date, album)| db::LatestRelease {
        name: &album.name,
        release_date,
        url: album.external_urls.spotify.as_deref(),
        artwork_url: album.images.first().map(|i| i.url.as_str()),
    });

    if let Err(err) =
        db::attach_release(&state.db.pool, normalized_name, release, popularity).await
    {
        tracing::warn!(error = %err, normalized_name, "artist enrichment: failed to backfill release info");
    }
}

/// Parses `album.release_date` at whatever precision Spotify actually
/// gave it (`release_date_precision`), defaulting the missing components
/// to the start of the period — e.g. a year-only release becomes Jan 1st
/// of that year. That default only ever pushes a release *further* from
/// "recent", never closer, so a coarse-precision release can't falsely
/// count as new — it can only ever be under-counted, which is the safe
/// direction for a "new release" flag.
fn parse_release_date(album: &SimplifiedAlbum) -> Option<chrono::NaiveDate> {
    use chrono::NaiveDate;

    match album.release_date_precision.as_str() {
        "day" => NaiveDate::parse_from_str(&album.release_date, "%Y-%m-%d").ok(),
        "month" => NaiveDate::parse_from_str(&format!("{}-01", album.release_date), "%Y-%m-%d").ok(),
        "year" => NaiveDate::parse_from_str(&format!("{}-01-01", album.release_date), "%Y-%m-%d")
            .ok(),
        _ => None,
    }
}

/// How many search results to check for an exact-normalized match, up
/// from just the top one. Still exact-match, so this is a pure recall
/// improvement with no new false-positive risk on its own — validated
/// against the real unmatched-artist corpus (see the `matching_eval` eval
/// harness): widening alone finds 7% of previously-unmatched names purely
/// because the right result wasn't ranked first.
const MAX_SEARCH_CANDIDATES: u8 = 10;

/// The name to search with, tried in order: the original name first, then
/// `cleaning::billing_variants(name)` (split multi-artist billing, strip
/// tour-name/city qualifiers) if the original doesn't verify. Combined
/// with the widened candidate pool, this is the "cleaned" strategy from
/// the eval harness — 23% recall on the real unmatched corpus, versus 7%
/// for widening alone.
fn name_variants(name: &str) -> Vec<String> {
    let mut variants = vec![name.to_string()];
    for v in cleaning::billing_variants(name) {
        if !variants.contains(&v) {
            variants.push(v);
        }
    }
    variants
}

/// Verified Apple Music catalog search: returns a match only when some
/// result among the top `MAX_SEARCH_CANDIDATES` normalizes to the same
/// string as the query (or a cleaned variant of it — see
/// `name_variants`), and isn't a bare genre word (see
/// `cleaning::is_bare_genre_word`) — Apple's search can return an
/// unrelated artist for an ambiguous or messy term, same rule
/// `apple_music::artwork_cache` already applies.
async fn try_apple_music_match(state: &AppState, name: &str) -> Option<(String, ArtistAttributes)> {
    let am = state.apple_music_client.as_ref()?;
    let dev_mgr = state.apple_dev_token.as_ref()?;
    let token = dev_mgr.get_token().await.ok()?;

    for variant in name_variants(name) {
        let normalized = normalize_artist_name(&variant);
        if normalized.is_empty() {
            continue;
        }
        let Ok(candidates) = am
            .search_artists(&token, &variant, MAX_SEARCH_CANDIDATES)
            .await
        else {
            continue;
        };
        for candidate in candidates {
            let Some(attrs) = candidate.attributes else {
                continue;
            };
            if normalize_artist_name(&attrs.name) == normalized
                && !cleaning::is_bare_genre_word(&attrs.name)
            {
                return Some((candidate.id, attrs));
            }
        }
    }
    None
}

/// Verified Spotify catalog search, under the same name-match rule as
/// `try_apple_music_match`. Always attempted alongside Apple Music (see
/// `resolve_fresh`) rather than only as a fallback — takes `SpotifyClient`/
/// `ClientCredentialsManager` directly rather than `&AppState` so it's
/// reusable outside a live request (see `spotify_backfill`, which has no
/// need for the rest of `AppState`'s fields).
pub(super) async fn try_spotify_match(
    spotify: &SpotifyClient,
    cc_manager: &ClientCredentialsManager,
    name: &str,
) -> Option<SpotifyArtist> {
    let token = cc_manager.get_token().await.ok()?;

    for variant in name_variants(name) {
        let normalized = normalize_artist_name(&variant);
        if normalized.is_empty() {
            continue;
        }
        let Ok(candidates) = spotify
            .search_artist(&token.access_token, &variant, MAX_SEARCH_CANDIDATES)
            .await
        else {
            continue;
        };
        if let Some(artist) = candidates.into_iter().find(|c| {
            normalize_artist_name(&c.name) == normalized && !cleaning::is_bare_genre_word(&c.name)
        }) {
            return Some(artist);
        }
    }
    None
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
        assert_eq!(
            deduped[0].ticketmaster_attraction_id.as_deref(),
            Some("K123")
        );
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
        assert_eq!(
            deduped[0].ticketmaster_attraction_id.as_deref(),
            Some("K123")
        );
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

    #[test]
    fn name_variants_always_tries_the_original_name_first() {
        let variants = name_variants("Casey Donahew in Deshler");
        assert_eq!(variants[0], "Casey Donahew in Deshler");
        assert!(variants.contains(&"Casey Donahew".to_string()));
    }

    #[test]
    fn name_variants_is_just_the_original_when_nothing_to_clean() {
        assert_eq!(name_variants("Role Model"), vec!["Role Model".to_string()]);
    }

    #[test]
    fn name_variants_has_no_duplicates() {
        let variants = name_variants("Zedd, Ganja White Night in Magna");
        let unique: std::collections::HashSet<_> = variants.iter().collect();
        assert_eq!(variants.len(), unique.len());
    }
}
