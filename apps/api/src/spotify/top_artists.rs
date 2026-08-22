//! Per-account cache of personalized-discovery matches, used to power the
//! "for you" match against nearby Ticketmaster events (see
//! `ticketmaster_stream`). Two tiers, from two providers:
//! - `Direct`: exact-normalized name match against the account's own
//!   Spotify artists — the union of top artists across all three of
//!   Spotify's own time ranges (~4 weeks, ~6 months, several years) plus
//!   the artists the account follows outright (a separate, non-listening-
//!   based signal — an artist you care about but don't stream heavily
//!   wouldn't show up in "top artists" at all). Each of the four sources
//!   degrades independently to "contributes nothing" if the connected
//!   token predates the scope it needs, rather than disabling
//!   personalization entirely — see `fetch_seed_source`.
//! - `Similar`: exact-normalized name match against Apple Music's
//!   similar-artists view for each seed artist from the above — widens the
//!   net beyond artists the account actually listens to or follows.
//!   Spotify deprecated its own related-artists endpoint for all apps
//!   without pre-existing extended quota access (Nov 2024); Apple's
//!   catalog equivalent needs no user auth and is already used elsewhere
//!   in this app (`apple_music::handlers`).
//!
//! Fetching any provider on every `/concerts/stream` request would be
//! wasteful and slow — that endpoint fires on every map pan/zoom — so
//! results are cached per account on a TTL instead.

use std::collections::{HashMap, HashSet};

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::Json;

use crate::apple_music::client::AppleMusicClient;
use crate::error::AppError;
use crate::spotify::client::{Artist, TopArtistsTimeRange, normalize_artist_name};
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

    // All four seed sources are independent, read-only lookups against the
    // same token -- concurrent, same reasoning as every other multi-call
    // fetch in this codebase (e.g. `artists::worker::resolve_fresh`).
    let (short, medium, long, followed) = tokio::join!(
        state
            .spotify
            .get_top_artists(&user_token, TopArtistsTimeRange::Short, 50),
        state
            .spotify
            .get_top_artists(&user_token, TopArtistsTimeRange::Medium, 50),
        state
            .spotify
            .get_top_artists(&user_token, TopArtistsTimeRange::Long, 50),
        state.spotify.get_followed_artists(&user_token, 50),
    );

    let short = fetch_seed_source(short, account_id, "user-top-read").await?;
    let medium = fetch_seed_source(medium, account_id, "user-top-read").await?;
    let long = fetch_seed_source(long, account_id, "user-top-read").await?;
    let followed = fetch_seed_source(followed, account_id, "user-follow-read").await?;

    let mut matches: MatchMap = MatchMap::new();
    let mut seen_normalized: HashSet<String> = HashSet::new();
    let mut seed_artists: Vec<Artist> = Vec::new();
    for artist in short.into_iter().chain(medium).chain(long).chain(followed) {
        let normalized = normalize_artist_name(&artist.name);
        matches.insert(normalized.clone(), MatchReason::Direct);
        // Dedup before the similar-artist expansion below -- the same
        // artist showing up in, say, both short- and long-term top artists
        // (common) shouldn't cost a second Apple Music lookup for it.
        if seen_normalized.insert(normalized) {
            seed_artists.push(artist);
        }
    }

    if let (Some(am), Some(dev_token_mgr)) = (
        state.apple_music_client.as_ref(),
        state.apple_dev_token.as_ref(),
    ) {
        match dev_token_mgr.get_token().await {
            Ok(dev_token) => {
                expand_via_similar_artists(am, &dev_token, &seed_artists, &mut matches).await;
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

/// Degrades a single seed source to "contributes nothing" when the
/// connected token predates the scope it needs (e.g. an account that
/// connected before `user-follow-read` was added), rather than failing the
/// whole personalization fetch over one missing scope — reconnecting
/// clears the cache immediately regardless (see `upsert_spotify_account`),
/// so this just means that source stays empty until then. Any other error
/// still propagates: a genuine API failure shouldn't get cached as "this
/// account simply has no artists" for the TTL's full 24 hours.
async fn fetch_seed_source(
    result: Result<Vec<Artist>, AppError>,
    account_id: uuid::Uuid,
    scope: &str,
) -> Result<Vec<Artist>, AppError> {
    match result {
        Ok(artists) => Ok(artists),
        Err(AppError::SpotifyApi {
            status: StatusCode::FORBIDDEN,
            ..
        }) => {
            tracing::info!(
                %account_id,
                scope,
                "spotify token missing scope, skipping this seed source until reconnect"
            );
            Ok(Vec::new())
        }
        Err(err) => Err(err),
    }
}

/// For each seed artist (the deduped union of the account's top artists
/// across all three Spotify time ranges plus its followed artists — see
/// `get_personalization_matches`), looks up its Apple Music catalog match
/// and folds in that artist's similar-artists names — under
/// `Similar { seed }`, where `seed` is the seed artist's display name —
/// unless a name is already present (a `Direct` entry always wins;
/// whichever `Similar` seed is found first wins over another, since
/// neither is more authoritative).
async fn expand_via_similar_artists(
    am: &AppleMusicClient,
    dev_token: &str,
    seed_artists: &[Artist],
    matches: &mut MatchMap,
) {
    let seeds: Vec<&str> = seed_artists.iter().map(|a| a.name.as_str()).collect();

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

#[cfg(test)]
mod tests {
    use super::*;

    fn artist(name: &str) -> Artist {
        Artist {
            id: "1".to_string(),
            name: name.to_string(),
            uri: String::new(),
            href: String::new(),
            external_urls: crate::spotify::client::ExternalUrls { spotify: None },
            images: vec![],
            artist_type: None,
            genres: vec![],
            popularity: 0,
        }
    }

    #[tokio::test]
    async fn fetch_seed_source_passes_through_a_successful_result() {
        let result = fetch_seed_source(
            Ok(vec![artist("Role Model")]),
            uuid::Uuid::nil(),
            "user-top-read",
        )
        .await;

        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn fetch_seed_source_degrades_a_missing_scope_to_an_empty_vec() {
        // A token that predates the scope this source needs (e.g. an
        // account connected before `user-follow-read` was added) must not
        // fail the whole personalization fetch over one missing scope.
        let result = fetch_seed_source(
            Err(AppError::SpotifyApi {
                status: StatusCode::FORBIDDEN,
                message: "insufficient scope".to_string(),
            }),
            uuid::Uuid::nil(),
            "user-follow-read",
        )
        .await;

        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn fetch_seed_source_propagates_a_non_forbidden_error() {
        // A genuine API failure (rate limit, outage) must not get cached
        // as "this account simply has no artists" for the TTL's full 24
        // hours -- see this function's own doc comment.
        let result = fetch_seed_source(
            Err(AppError::SpotifyApi {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                message: "spotify is down".to_string(),
            }),
            uuid::Uuid::nil(),
            "user-top-read",
        )
        .await;

        assert!(result.is_err());
    }
}
