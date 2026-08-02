//! Per-account cache of personalized-discovery matches, used to power the
//! "for you" match against nearby Ticketmaster events (see
//! `ticketmaster_stream`). Two tiers, from two providers:
//! - `Direct`: exact-normalized name match against the account's Spotify top
//!   artists (~6 month window).
//! - `Similar`: exact-normalized name match against Apple Music's
//!   similar-artists view for each top artist — widens the net beyond
//!   artists the account actually listens to. Spotify deprecated its own
//!   related-artists endpoint for all apps without pre-existing extended
//!   quota access (Nov 2024); Apple's catalog equivalent needs no user auth
//!   and is already used elsewhere in this app (`apple_music::handlers`).
//!
//! Fetching either provider on every `/concerts/stream` request would be
//! wasteful and slow — that endpoint fires on every map pan/zoom — so
//! results are cached per account on a TTL instead.

use std::collections::HashMap;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::apple_music::client::AppleMusicClient;
use crate::error::AppError;
use crate::spotify::client::{TopArtistsTimeRange, normalize_artist_name};
use crate::state::AppState;

const CACHE_TTL: Duration = Duration::hours(24);
/// Caps how many similar-artist lookups run at once against Apple's API —
/// there can be up to 50 top artists, each needing a search + similar-artists
/// call, and this isn't a request users are waiting synchronously on (it's
/// cached), so favor being a polite API citizen over raw throughput.
const MAX_CONCURRENT_SIMILAR_LOOKUPS: usize = 8;

type MatchMap = HashMap<String, MatchReason>;

/// Why a Ticketmaster attraction name matched the cache — determines the
/// "for you" popover copy on the frontend (direct match vs. similar-to).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum MatchReason {
    Direct,
    Similar { seed: String },
}

struct CacheRow {
    matches: Json<MatchMap>,
    fetched_at: chrono::DateTime<Utc>,
}

/// Returns the exact-normalized personalization matches for `account_id`,
/// refreshing from Spotify (and, if configured, Apple Music) if the cache is
/// missing or stale.
///
/// Returns an empty map — never an error — if the account has no Spotify
/// connection, since callers use this to gate an optional feature
/// (personalization) that must never break the underlying event stream.
pub(crate) async fn get_personalization_matches(
    state: &AppState,
    account_id: uuid::Uuid,
) -> Result<MatchMap, AppError> {
    let cached = sqlx::query_as!(
        CacheRow,
        r#"
        select matches as "matches: Json<MatchMap>", fetched_at
        from spotify_top_artist_cache
        where account_id = $1
        "#,
        account_id
    )
    .fetch_optional(&state.db.pool)
    .await?;

    if let Some(row) = &cached
        && row.fetched_at > Utc::now() - CACHE_TTL
    {
        return Ok(row.matches.0.clone());
    }

    let user_token = match super::token::get_valid_spotify_token(state, account_id).await {
        Ok(token) => token,
        Err(AppError::AuthRequired(_)) => return Ok(MatchMap::new()),
        Err(err) => return Err(err),
    };

    let top_artists = match state
        .spotify
        .get_top_artists(&user_token, TopArtistsTimeRange::Medium, 50)
        .await
    {
        Ok(artists) => artists,
        Err(AppError::SpotifyApi {
            status: StatusCode::FORBIDDEN,
            ..
        }) => {
            // Token predates the `user-top-read` scope (added after this
            // account connected). Cache the empty result so we don't retry
            // Spotify — and warn on every request — until the account
            // reconnects, which clears this cache immediately (see
            // `upsert_spotify_account`).
            tracing::info!(
                %account_id,
                "spotify token missing user-top-read scope, disabling personalization until reconnect"
            );
            cache_matches(state, account_id, &MatchMap::new()).await?;
            return Ok(MatchMap::new());
        }
        Err(err) => return Err(err),
    };

    let mut matches: MatchMap = MatchMap::new();
    for artist in &top_artists {
        matches.insert(normalize_artist_name(&artist.name), MatchReason::Direct);
    }

    if let (Some(am), Some(dev_token_mgr)) = (
        state.apple_music_client.as_ref(),
        state.apple_dev_token.as_ref(),
    ) {
        match dev_token_mgr.get_token().await {
            Ok(dev_token) => {
                expand_via_similar_artists(am, &dev_token, &top_artists, &mut matches).await;
            }
            Err(err) => {
                tracing::warn!(
                    error = %err,
                    "failed to fetch Apple Music developer token, skipping similar-artist expansion"
                );
            }
        }
    }

    cache_matches(state, account_id, &matches).await?;

    Ok(matches)
}

/// For each Spotify top artist, looks up its Apple Music catalog match and
/// folds in that artist's similar-artists names — under `Similar { seed }`,
/// where `seed` is the top artist's display name — unless a name is already
/// present (a `Direct` entry always wins; whichever `Similar` seed is found
/// first wins over another, since neither is more authoritative).
async fn expand_via_similar_artists(
    am: &AppleMusicClient,
    dev_token: &str,
    top_artists: &[crate::spotify::client::Artist],
    matches: &mut MatchMap,
) {
    let seeds: Vec<&str> = top_artists.iter().map(|a| a.name.as_str()).collect();

    // Chunked (rather than `stream::buffer_unordered`, which doesn't play
    // well with closures borrowing `am`/`dev_token` across an await point)
    // to cap how many concurrent requests hit Apple's API at once.
    for chunk in seeds.chunks(MAX_CONCURRENT_SIMILAR_LOOKUPS) {
        let results = futures::future::join_all(
            chunk
                .iter()
                .map(|seed| fetch_similar_artist_names(am, dev_token, seed)),
        )
        .await;

        for (seed, result) in chunk.iter().zip(results) {
            let similar_names = match result {
                Ok(names) => names,
                Err(err) => {
                    tracing::debug!(seed = %seed, error = %err, "skipping similar-artist lookup for seed");
                    continue;
                }
            };
            for name in similar_names {
                matches
                    .entry(normalize_artist_name(&name))
                    .or_insert_with(|| MatchReason::Similar {
                        seed: seed.to_string(),
                    });
            }
        }
    }
}

/// Searches Apple's catalog for `seed_name`, validates the best result is an
/// exact-normalized match (a fuzzy mismatch here would seed the "for you"
/// pool from the wrong artist entirely), then returns that artist's
/// similar-artist names via Apple's `similar-artists` view.
async fn fetch_similar_artist_names(
    am: &AppleMusicClient,
    dev_token: &str,
    seed_name: &str,
) -> Result<Vec<String>, AppError> {
    let candidates = am.search_artists(dev_token, seed_name, 1).await?;
    let matched = candidates
        .into_iter()
        .find(|a| {
            a.attributes.as_ref().is_some_and(|attrs| {
                normalize_artist_name(&attrs.name) == normalize_artist_name(seed_name)
            })
        })
        .ok_or_else(|| AppError::ArtistNotFound(seed_name.to_string()))?;

    let similar = am.get_similar_artists(dev_token, &matched.id).await?;

    Ok(similar
        .into_iter()
        .filter_map(|a| a.attributes.map(|attrs| attrs.name))
        .collect())
}

async fn cache_matches(
    state: &AppState,
    account_id: uuid::Uuid,
    matches: &MatchMap,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        insert into spotify_top_artist_cache (account_id, matches, fetched_at)
        values ($1, $2, now())
        on conflict (account_id) do update set
            matches = excluded.matches,
            fetched_at = excluded.fetched_at
        "#,
        account_id,
        Json(matches) as _,
    )
    .execute(&state.db.pool)
    .await?;

    Ok(())
}
