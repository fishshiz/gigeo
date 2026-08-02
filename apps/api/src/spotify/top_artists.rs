//! Per-account cache of exact-normalized Spotify top-artist names, used to
//! power the personalized discovery match against nearby Ticketmaster events
//! (see `ticketmaster_stream`). Fetching `/me/top/artists` on every
//! `/concerts/stream` request would be wasteful — that endpoint fires on
//! every map pan/zoom — so results are cached per account on a TTL instead.

use std::collections::HashSet;

use chrono::{Duration, Utc};

use crate::error::AppError;
use crate::spotify::client::{TopArtistsTimeRange, normalize_artist_name};
use crate::state::AppState;

const CACHE_TTL: Duration = Duration::hours(24);

struct CacheRow {
    normalized_names: Vec<String>,
    fetched_at: chrono::DateTime<Utc>,
}

/// Returns the exact-normalized top-artist names for `account_id` (~6 month
/// window), refreshing from Spotify if the cache is missing or stale.
///
/// Returns an empty set — never an error — if the account has no Spotify
/// connection, since callers use this to gate an optional feature
/// (personalization) that must never break the underlying event stream.
pub(crate) async fn get_top_artist_names(
    state: &AppState,
    account_id: uuid::Uuid,
) -> Result<HashSet<String>, AppError> {
    let cached = sqlx::query_as!(
        CacheRow,
        r#"
        select normalized_names, fetched_at
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
        return Ok(row.normalized_names.iter().cloned().collect());
    }

    let user_token = match super::token::get_valid_spotify_token(state, account_id).await {
        Ok(token) => token,
        Err(AppError::AuthRequired(_)) => return Ok(HashSet::new()),
        Err(err) => return Err(err),
    };

    let artists = state
        .spotify
        .get_top_artists(&user_token, TopArtistsTimeRange::Medium, 50)
        .await?;

    let normalized_names: Vec<String> = artists
        .iter()
        .map(|a| normalize_artist_name(&a.name))
        .collect();

    sqlx::query!(
        r#"
        insert into spotify_top_artist_cache (account_id, normalized_names, fetched_at)
        values ($1, $2, now())
        on conflict (account_id) do update set
            normalized_names = excluded.normalized_names,
            fetched_at = excluded.fetched_at
        "#,
        account_id,
        &normalized_names,
    )
    .execute(&state.db.pool)
    .await?;

    Ok(normalized_names.into_iter().collect())
}
