//! Persistence for the `artists` table. `matched_via` is stored as the
//! `music_provider` enum in Postgres (reusing the existing type rather than
//! inventing a new one) but handled as a plain `"apple_music" | "spotify"`
//! string on the Rust side via an explicit `::text`/`::music_provider`
//! cast, so this stays a plain string match in `worker.rs` instead of
//! needing its own `sqlx::Type` mapping.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::types::Json;

use super::SimilarArtist;

pub(super) struct ArtistRow {
    pub matched_via: Option<String>,
    pub apple_music_id: Option<String>,
    pub spotify_id: Option<String>,
    pub last_attempted_at: DateTime<Utc>,
    pub enrichment_refreshed_at: Option<DateTime<Utc>>,
}

pub(super) async fn get_artist(
    pool: &PgPool,
    normalized_name: &str,
) -> Result<Option<ArtistRow>, sqlx::Error> {
    sqlx::query_as!(
        ArtistRow,
        r#"
        select
            matched_via::text as "matched_via: String",
            apple_music_id,
            spotify_id,
            last_attempted_at,
            enrichment_refreshed_at
        from artists
        where normalized_name = $1
        "#,
        normalized_name
    )
    .fetch_optional(pool)
    .await
}

/// Fields for a freshly (or re-)resolved match. Identity fields
/// (`apple_music_id`/`spotify_id`/`matched_via`/`matched_at`) only ever get
/// set once — `on conflict` coalesces them against the existing row rather
/// than overwriting, since `worker.rs` never calls this for a row it
/// already found matched (see `resolve_one`'s dispatch). Everything else
/// (artwork, genres, similar artists) is refreshed unconditionally.
pub(super) struct MatchedArtist<'a> {
    pub normalized_name: &'a str,
    pub display_name: &'a str,
    pub apple_music_id: Option<&'a str>,
    pub spotify_id: Option<&'a str>,
    pub ticketmaster_attraction_id: Option<&'a str>,
    /// `"apple_music"` or `"spotify"` — cast to `music_provider` in SQL.
    pub matched_via: &'a str,
    pub artwork_url: Option<&'a str>,
    pub artwork_bg_color: Option<&'a str>,
    pub apple_music_url: Option<&'a str>,
    pub spotify_url: Option<&'a str>,
    pub genres: &'a [String],
    pub similar_artists: &'a [SimilarArtist],
}

pub(super) async fn upsert_matched(pool: &PgPool, row: MatchedArtist<'_>) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into artists (
            normalized_name, display_name,
            apple_music_id, spotify_id, ticketmaster_attraction_id,
            matched_via, matched_at,
            artwork_url, artwork_bg_color, apple_music_url, spotify_url,
            genres, similar_artists,
            last_attempted_at, enrichment_refreshed_at
        )
        values (
            $1, $2, $3, $4, $5, $6::text::music_provider, now(),
            $7, $8, $9, $10, $11, $12, now(), now()
        )
        on conflict (normalized_name) do update set
            display_name = excluded.display_name,
            apple_music_id = coalesce(artists.apple_music_id, excluded.apple_music_id),
            spotify_id = coalesce(artists.spotify_id, excluded.spotify_id),
            ticketmaster_attraction_id = coalesce(excluded.ticketmaster_attraction_id, artists.ticketmaster_attraction_id),
            matched_via = coalesce(artists.matched_via, excluded.matched_via),
            matched_at = coalesce(artists.matched_at, excluded.matched_at),
            artwork_url = excluded.artwork_url,
            artwork_bg_color = excluded.artwork_bg_color,
            apple_music_url = excluded.apple_music_url,
            spotify_url = excluded.spotify_url,
            genres = excluded.genres,
            similar_artists = excluded.similar_artists,
            last_attempted_at = now(),
            enrichment_refreshed_at = now()
        "#,
        row.normalized_name,
        row.display_name,
        row.apple_music_id,
        row.spotify_id,
        row.ticketmaster_attraction_id,
        row.matched_via,
        row.artwork_url,
        row.artwork_bg_color,
        row.apple_music_url,
        row.spotify_url,
        row.genres,
        Json(row.similar_artists) as _,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

/// Records a no-confident-match attempt (an artwork-cache-style "cache the
/// miss too", so a noisy PredictHQ title isn't re-searched on every
/// request that includes it) — see `worker::UNMATCHED_RETRY_COOLDOWN` for
/// when it's retried.
pub(super) async fn upsert_tombstone(
    pool: &PgPool,
    normalized_name: &str,
    display_name: &str,
    ticketmaster_attraction_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into artists (normalized_name, display_name, ticketmaster_attraction_id, last_attempted_at)
        values ($1, $2, $3, now())
        on conflict (normalized_name) do update set
            ticketmaster_attraction_id = coalesce(excluded.ticketmaster_attraction_id, artists.ticketmaster_attraction_id),
            last_attempted_at = now()
        "#,
        normalized_name,
        display_name,
        ticketmaster_attraction_id,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

/// Bumps `last_attempted_at` without touching anything else — used when a
/// dynamic-field refresh attempt fails (e.g. the provider call errors), so
/// it isn't retried on literally the next request.
pub(super) async fn touch_last_attempted(
    pool: &PgPool,
    normalized_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update artists set last_attempted_at = now() where normalized_name = $1",
        normalized_name
    )
    .execute(pool)
    .await
    .map(|_| ())
}

/// Refreshes only the fields expected to go stale on their own (genres,
/// similar artists) for an already-matched row — identity and artwork are
/// left untouched, per ADR-0001.
pub(super) async fn refresh_dynamic(
    pool: &PgPool,
    normalized_name: &str,
    genres: &[String],
    similar_artists: &[SimilarArtist],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        update artists
        set genres = $2, similar_artists = $3, last_attempted_at = now(), enrichment_refreshed_at = now()
        where normalized_name = $1
        "#,
        normalized_name,
        genres,
        Json(similar_artists) as _,
    )
    .execute(pool)
    .await
    .map(|_| ())
}
