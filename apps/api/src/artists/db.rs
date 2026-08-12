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
///
/// `apple_music_id`/`spotify_id`/`ticketmaster_attraction_id` each have
/// their own partial unique index (`migrations/006_artists.sql`) enforcing
/// one canonical row per real-world artist identity — deliberate, per
/// ADR-0001. But more than one `normalized_name` can legitimately resolve
/// to the *same* identity: e.g. a plain Ticketmaster attraction name
/// alongside PredictHQ's event-title-as-performer-name fallback for the
/// same real artist (confirmed live — "LCD Soundsystem" and several others
/// hit exactly this). `attach_enrichment` looks up by `normalized_name`
/// only, so a losing name variant still needs *a* row to ever show
/// enrichment, even though it can't hold the identity field's unique slot
/// itself — see `upsert_matched`'s retry-without-that-field fallback.
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

/// Upserts a resolved match, retrying with an already-claimed identity
/// field cleared if it collides with a different `normalized_name`'s row
/// (see `MatchedArtist`'s doc comment). Bounded to at most one retry per
/// identity field — `apple_music_id`, then `spotify_id`, then
/// `ticketmaster_attraction_id` — so this always terminates: each retry
/// permanently nulls the offending field out of `row` before trying again,
/// and there are only three such fields to exhaust.
pub(super) async fn upsert_matched(
    pool: &PgPool,
    mut row: MatchedArtist<'_>,
) -> Result<(), sqlx::Error> {
    loop {
        match execute_upsert_matched(pool, &row).await {
            Ok(()) => return Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                match db_err.constraint() {
                    Some("artists_apple_music_id_idx") if row.apple_music_id.is_some() => {
                        row.apple_music_id = None;
                    }
                    Some("artists_spotify_id_idx") if row.spotify_id.is_some() => {
                        row.spotify_id = None;
                    }
                    Some("artists_ticketmaster_attraction_id_idx")
                        if row.ticketmaster_attraction_id.is_some() =>
                    {
                        row.ticketmaster_attraction_id = None;
                    }
                    _ => return Err(sqlx::Error::Database(db_err)),
                }
            }
            Err(err) => return Err(err),
        }
    }
}

async fn execute_upsert_matched(pool: &PgPool, row: &MatchedArtist<'_>) -> Result<(), sqlx::Error> {
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

pub(super) struct SimilarArtistsStatus {
    pub apple_music_id: Option<String>,
    pub has_similar_artists: bool,
}

/// Whether `normalized_name` already has similar artists persisted, and
/// its Apple Music id to fetch them with if not -- `None` when the row
/// doesn't exist or wasn't matched via Apple Music (similar artists only
/// ever come from Apple, see `worker::resolve_fresh`). Backs
/// `worker::backfill_similar_artists`.
pub(super) async fn get_similar_artists(
    pool: &PgPool,
    normalized_name: &str,
) -> Result<Option<SimilarArtistsStatus>, sqlx::Error> {
    sqlx::query_as!(
        SimilarArtistsStatus,
        r#"
        select
            apple_music_id,
            similar_artists != '[]'::jsonb as "has_similar_artists!"
        from artists
        where normalized_name = $1 and matched_via = 'apple_music'::music_provider
        "#,
        normalized_name
    )
    .fetch_optional(pool)
    .await
}

/// Persists a similar-artists backfill without touching genres or bumping
/// `enrichment_refreshed_at` -- deliberately narrower than `refresh_dynamic`,
/// since this only fills a gap `resolve_fresh` left on purpose rather than
/// refreshing an already-complete row on its normal TTL.
pub(super) async fn attach_similar_artists(
    pool: &PgPool,
    normalized_name: &str,
    similar_artists: &[SimilarArtist],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        update artists
        set similar_artists = $2, last_attempted_at = now()
        where normalized_name = $1
        "#,
        normalized_name,
        Json(similar_artists) as _,
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

/// Narrowly attaches a Spotify identity to an already-matched row without
/// touching Apple identity/artwork/genres/similar_artists — the write path
/// for the one-off Spotify backfill (see `spotify_backfill`), for a row
/// that was matched via Apple Music before Spotify was looked up alongside
/// it too. Guarded by `where spotify_id is null`: unlike
/// `upsert_matched`'s unconditional `spotify_url = excluded.spotify_url`,
/// this must never clobber a value another path already wrote (e.g. an
/// overlapping backfill run). Deliberately doesn't touch
/// `enrichment_refreshed_at` — that drives the dynamic-field-freshness
/// check in `worker::resolve_one`, and bumping it here would silently push
/// out this row's next genre/similar-artist refresh as a side effect of an
/// unrelated write.
///
/// Only consumed by the `#[cfg(test)]`-gated `spotify_backfill` tool today,
/// so a plain non-test build sees no caller.
#[allow(dead_code)]
pub(super) async fn attach_spotify(
    pool: &PgPool,
    normalized_name: &str,
    spotify_id: &str,
    spotify_url: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        update artists
        set spotify_id = $2, spotify_url = $3, last_attempted_at = now()
        where normalized_name = $1 and spotify_id is null
        "#,
        normalized_name,
        spotify_id,
        spotify_url,
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
