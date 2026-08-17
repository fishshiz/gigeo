//! Periodic playlist updater: polls the `playlist` table for rows whose
//! `next_update_at` is due, refreshes each one's tracks from currently-
//! nearby Ticketmaster events, and records the outcome in
//! `playlist_update_run`.
//!
//! Covers both providers, but not symmetrically. Spotify playlists get
//! full additive/destructive updates via a refreshable OAuth token.
//! Apple Music playlists are always additive (the public Apple Music API
//! has no track-removal endpoint — see the Phase 0 plan) and use
//! whatever Music User Token is on file with no refresh flow; an
//! expired/revoked token surfaces as an auth failure and deactivates the
//! playlist the same way a 404 does on Spotify's side.

use std::collections::HashSet;
use std::time::Duration as StdDuration;

use chrono::{Duration as ChronoDuration, Utc};
use sqlx::PgPool;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::apple_music::apple_handlers::get_apple_music_account;
use crate::error::AppError;
use crate::services::playlist_builder::{
    PlaylistUpdateMode, build_bulleted_description, find_artist_events_near,
    find_artist_names_near, search_tracks_for_artists, search_tracks_for_artists_apple,
};
use crate::spotify::token::get_valid_spotify_token;
use crate::state::AppState;

const POLL_INTERVAL_SECS: u64 = 300; // 5 min
const CLAIM_GRACE: ChronoDuration = ChronoDuration::minutes(15);
const FAILURE_RETRY_DELAY: ChronoDuration = ChronoDuration::hours(1);
const MAX_PLAYLISTS_PER_TICK: usize = 25;
const SEARCH_RADIUS_MILES: u8 = 25;
const EVENT_WINDOW_DAYS: i64 = 7;
const TRACKS_PER_ARTIST: u8 = 3;
/// Upper bound on how many tracks a single update run touches, regardless
/// of how many nearby artists/tracks were found. Plain truncation for now;
/// once genre prioritization exists (playlist_genre_priority is already in
/// the schema, unused), this is the natural place to select tracks by
/// priority instead of just taking the first N.
const MAX_TRACKS_PER_RUN: usize = 30;

/// Entry point called from `build_app()`. Spawns the background poll loop.
pub fn spawn(state: AppState) -> tokio::task::JoinHandle<()> {
    tokio::spawn(run_loop(state))
}

async fn run_loop(state: AppState) {
    let mut interval = tokio::time::interval(StdDuration::from_secs(POLL_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        let n = run_cycle(&state).await;
        if n > 0 {
            info!(count = n, "playlist updater: processed due playlists");
        }
    }
}

/// Claims and processes up to `MAX_PLAYLISTS_PER_TICK` due playlists.
/// Returns the number processed. A single playlist's failure never stops
/// the rest of the tick — each playlist's errors are caught and recorded.
pub async fn run_cycle(state: &AppState) -> usize {
    let mut processed = 0;

    for _ in 0..MAX_PLAYLISTS_PER_TICK {
        let claim_until = Utc::now() + CLAIM_GRACE;
        match claim_due_playlist(&state.db.pool, claim_until).await {
            Ok(Some(claimed)) => {
                processed += 1;
                update_one_playlist(state, claimed).await;
            }
            Ok(None) => break, // nothing left due this tick
            Err(e) => {
                error!(error = %e, "playlist updater: claim query failed");
                break;
            }
        }
    }

    processed
}

struct ClaimedPlaylist {
    id: Uuid,
    account_id: Uuid,
    provider_playlist_id: String,
    geohash: String,
    update_mode: PlaylistUpdateMode,
    update_cadence_days: i16,
    /// `"spotify"` or `"apple_music"` — plain string, following the same
    /// `music_provider::text` cast convention `artists/db.rs` already
    /// established rather than introducing a dedicated Rust enum.
    provider: String,
}

/// Atomically claims one due playlist: `FOR UPDATE SKIP LOCKED` picks and
/// locks a single row (skipping any already locked by a concurrent tick),
/// then the outer `UPDATE` bumps `next_update_at` forward by a short grace
/// window so nothing else claims it while this update is in flight. This
/// is the standard Postgres job-queue claim pattern. Joins `account` (via
/// `UPDATE ... FROM`) purely to read `provider` alongside the claimed row.
async fn claim_due_playlist(
    pool: &PgPool,
    claim_until: chrono::DateTime<Utc>,
) -> Result<Option<ClaimedPlaylist>, sqlx::Error> {
    sqlx::query_as!(
        ClaimedPlaylist,
        r#"
        update playlist
        set next_update_at = $1
        from account
        where account.id = playlist.account_id
          and playlist.id = (
            select id from playlist
            where is_active
              and deleted_at is null
              and (next_update_at is null or next_update_at <= now())
            order by next_update_at nulls first
            limit 1
            for update skip locked
        )
        returning
            playlist.id, playlist.account_id, playlist.provider_playlist_id, playlist.geohash,
            playlist.update_mode as "update_mode: PlaylistUpdateMode",
            playlist.update_cadence_days,
            account.provider::text as "provider!"
        "#,
        claim_until
    )
    .fetch_optional(pool)
    .await
}

/// Runs one full update cycle for a single claimed playlist: resolve
/// tokens -> find nearby artists -> search tracks -> diff against current
/// contents -> mutate Spotify -> record the run -> finalize the playlist's
/// scheduling fields.
async fn update_one_playlist(state: &AppState, p: ClaimedPlaylist) {
    let run_id = match insert_pending_run(&state.db.pool, p.id, p.update_mode).await {
        Ok(id) => id,
        Err(e) => {
            error!(playlist_id = %p.id, error = %e, "failed to insert playlist_update_run");
            return;
        }
    };

    match do_update(state, &p).await {
        Ok(outcome) => {
            if let Err(e) = finish_run_success(&state.db.pool, run_id, &outcome).await {
                error!(playlist_id = %p.id, error = %e, "failed to finalize playlist_update_run (success)");
            }
            if let Err(e) =
                finalize_playlist_success(&state.db.pool, p.id, p.update_cadence_days).await
            {
                error!(playlist_id = %p.id, error = %e, "failed to finalize playlist after success");
            }
        }
        Err(e) => {
            warn!(playlist_id = %p.id, error = %e, "playlist update failed");
            let deactivate = if p.provider == "apple_music" {
                // No 404-equivalent "gone" signal from Apple; a 401/403 or
                // AuthRequired (missing account row / no token) is as
                // close as it gets to unrecoverable, since there's no
                // refresh flow to retry with.
                matches!(&e, AppError::AppleMusicApi { status, .. } if status.as_u16() == 401 || status.as_u16() == 403)
                    || matches!(&e, AppError::AuthRequired(_))
            } else {
                matches!(&e, AppError::SpotifyApi { status, .. } if status.as_u16() == 404)
            };
            if let Err(e) = finish_run_failure(&state.db.pool, run_id, &e.to_string()).await {
                error!(playlist_id = %p.id, error = %e, "failed to finalize playlist_update_run (failure)");
            }
            if let Err(e) = finalize_playlist_failure(&state.db.pool, p.id, deactivate).await {
                error!(playlist_id = %p.id, error = %e, "failed to finalize playlist after failure");
            }
        }
    }
}

struct UpdateOutcome {
    tracks_added: i32,
    tracks_removed: i32,
    artist_names: Vec<String>,
}

/// Splits `new` into the URIs not already present in `current`, plus the
/// added/removed counts relative to `current` (destructive mode replaces
/// wholesale, so both counts matter there; additive mode only ever adds,
/// so callers should treat its `tracks_removed` as always 0 regardless of
/// what this returns).
fn diff_uris(current: &[String], new: &[String]) -> (Vec<String>, i32, i32) {
    let current_set: HashSet<&String> = current.iter().collect();
    let new_set: HashSet<&String> = new.iter().collect();

    let to_add: Vec<String> = new
        .iter()
        .filter(|u| !current_set.contains(u))
        .cloned()
        .collect();
    let added = new_set.difference(&current_set).count() as i32;
    let removed = current_set.difference(&new_set).count() as i32;

    (to_add, added, removed)
}

/// Apple Music's update path: always additive (see module doc) — finds
/// new tracks and adds whatever isn't already in the playlist, never
/// removes anything. No refresh flow for the stored Music User Token; a
/// missing account row or an auth error from Apple propagates up to
/// `update_one_playlist`, which treats it as unrecoverable.
async fn do_update_apple(state: &AppState, p: &ClaimedPlaylist) -> Result<UpdateOutcome, AppError> {
    let (coord, _, _) = geohash::decode(&p.geohash)
        .map_err(|e| AppError::Internal(format!("geohash decode failed: {e}")))?;

    let am = state
        .apple_music_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music client not configured".into()))?;
    let dev_token = state
        .apple_dev_token
        .as_ref()
        .ok_or_else(|| {
            AppError::Internal("Apple Music developer token manager not configured".into())
        })?
        .get_token()
        .await?;
    let account = get_apple_music_account(&state.db.pool, p.account_id)
        .await?
        .ok_or_else(|| AppError::AuthRequired("Apple Music account no longer connected.".into()))?;

    let artist_names = find_artist_names_near(
        state,
        coord.y,
        coord.x,
        SEARCH_RADIUS_MILES,
        EVENT_WINDOW_DAYS,
    )
    .await?;

    if artist_names.is_empty() {
        return Ok(UpdateOutcome {
            tracks_added: 0,
            tracks_removed: 0,
            artist_names: vec![],
        });
    }

    let track_results =
        search_tracks_for_artists_apple(state, &dev_token, &artist_names, TRACKS_PER_ARTIST)
            .await?;

    let mut seen = HashSet::new();
    let new_song_ids: Vec<String> = track_results
        .into_iter()
        .filter(|t| seen.insert(t.uri.clone()))
        .map(|t| t.uri)
        .collect();

    let current_ids: HashSet<String> = am
        .get_library_playlist_tracks(
            &dev_token,
            &account.music_user_token,
            &p.provider_playlist_id,
        )
        .await?
        .into_iter()
        .map(|s| s.id)
        .collect();

    let mut to_add: Vec<String> = new_song_ids
        .into_iter()
        .filter(|id| !current_ids.contains(id))
        .collect();
    to_add.truncate(MAX_TRACKS_PER_RUN);

    for chunk in to_add.chunks(100) {
        am.add_tracks_to_playlist(
            &dev_token,
            &account.music_user_token,
            &p.provider_playlist_id,
            chunk,
        )
        .await?;
    }

    Ok(UpdateOutcome {
        tracks_added: to_add.len() as i32,
        tracks_removed: 0,
        artist_names,
    })
}

async fn do_update(state: &AppState, p: &ClaimedPlaylist) -> Result<UpdateOutcome, AppError> {
    match p.provider.as_str() {
        "apple_music" => do_update_apple(state, p).await,
        _ => do_update_spotify(state, p).await,
    }
}

async fn do_update_spotify(
    state: &AppState,
    p: &ClaimedPlaylist,
) -> Result<UpdateOutcome, AppError> {
    let (coord, _, _) = geohash::decode(&p.geohash)
        .map_err(|e| AppError::Internal(format!("geohash decode failed: {e}")))?;

    let user_token = get_valid_spotify_token(state, p.account_id).await?;
    let cc_token = state.cc_manager.get_token().await?;

    let artist_events = find_artist_events_near(
        state,
        coord.y,
        coord.x,
        SEARCH_RADIUS_MILES,
        EVENT_WINDOW_DAYS,
    )
    .await?;
    let artist_names: Vec<String> = artist_events.iter().map(|a| a.name.clone()).collect();

    if artist_names.is_empty() {
        return Ok(UpdateOutcome {
            tracks_added: 0,
            tracks_removed: 0,
            artist_names: vec![],
        });
    }

    let track_results = search_tracks_for_artists(
        state,
        &cc_token.access_token,
        &artist_names,
        TRACKS_PER_ARTIST,
    )
    .await?;

    // De-dupe while preserving first-seen order (the same track can
    // surface via multiple artists, e.g. a featured collaboration).
    let mut seen = HashSet::new();
    let new_uris: Vec<String> = track_results
        .into_iter()
        .filter(|t| seen.insert(t.uri.clone()))
        .map(|t| t.uri)
        .collect();

    let current_uris = state
        .spotify
        .get_playlist_tracks(&user_token, &p.provider_playlist_id)
        .await?;

    match p.update_mode {
        PlaylistUpdateMode::Additive => {
            // Cap after diffing, not before: capping the raw candidate pool
            // first could mean most of it is already in the playlist from a
            // prior cycle, leaving to_add short even when plenty of other
            // fresh tracks exist beyond the cap.
            let (mut to_add, _, _) = diff_uris(&current_uris, &new_uris);
            to_add.truncate(MAX_TRACKS_PER_RUN);

            for chunk in to_add.chunks(100) {
                state
                    .spotify
                    .add_items_to_playlist(&user_token, &p.provider_playlist_id, chunk)
                    .await?;
            }
            Ok(UpdateOutcome {
                tracks_added: to_add.len() as i32,
                tracks_removed: 0,
                artist_names,
            })
        }
        PlaylistUpdateMode::Destructive => {
            // Destructive is a full replace, so the cap bounds the
            // resulting playlist size rather than an increment — capped
            // before diffing so tracks_added/tracks_removed reflect what
            // actually got written.
            let mut capped_new_uris = new_uris;
            capped_new_uris.truncate(MAX_TRACKS_PER_RUN);

            if capped_new_uris.is_empty() {
                // Never wipe a playlist because this cycle found zero
                // tracks (e.g. a transient Ticketmaster hiccup) — leave it
                // untouched rather than destroying its contents.
                return Ok(UpdateOutcome {
                    tracks_added: 0,
                    tracks_removed: 0,
                    artist_names,
                });
            }

            let (_, added, removed) = diff_uris(&current_uris, &capped_new_uris);

            let mut chunks = capped_new_uris.chunks(100);
            if let Some(first) = chunks.next() {
                state
                    .spotify
                    .replace_playlist_items(&user_token, &p.provider_playlist_id, first)
                    .await?;
            }
            for chunk in chunks {
                state
                    .spotify
                    .add_items_to_playlist(&user_token, &p.provider_playlist_id, chunk)
                    .await?;
            }

            // A destructive replace invalidates whatever the description
            // said before (it described tracks that no longer exist), so
            // it's always regenerated here — never left stale, and never
            // left as a user-authored description that's now describing
            // the wrong playlist.
            let description = build_bulleted_description(&artist_events);
            state
                .spotify
                .update_playlist_details(
                    &user_token,
                    &p.provider_playlist_id,
                    None,
                    None,
                    Some(&description),
                )
                .await?;

            Ok(UpdateOutcome {
                tracks_added: added,
                tracks_removed: removed,
                artist_names,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// playlist_update_run bookkeeping
// ---------------------------------------------------------------------------

async fn insert_pending_run(
    pool: &PgPool,
    playlist_id: Uuid,
    mode: PlaylistUpdateMode,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        insert into playlist_update_run (playlist_id, status, update_mode)
        values ($1, 'running', $2)
        returning id
        "#,
        playlist_id,
        mode as _,
    )
    .fetch_one(pool)
    .await
}

async fn finish_run_success(
    pool: &PgPool,
    run_id: Uuid,
    outcome: &UpdateOutcome,
) -> Result<(), sqlx::Error> {
    let artists_added = serde_json::to_value(&outcome.artist_names).unwrap_or_default();
    sqlx::query!(
        r#"
        update playlist_update_run
        set status = 'success', finished_at = now(),
            tracks_added = $2, tracks_removed = $3, artists_added = $4
        where id = $1
        "#,
        run_id,
        outcome.tracks_added,
        outcome.tracks_removed,
        artists_added,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

async fn finish_run_failure(pool: &PgPool, run_id: Uuid, message: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        update playlist_update_run
        set status = 'failed', finished_at = now(), error_message = $2
        where id = $1
        "#,
        run_id,
        message,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

async fn finalize_playlist_success(
    pool: &PgPool,
    playlist_id: Uuid,
    cadence_days: i16,
) -> Result<(), sqlx::Error> {
    let next_update_at = Utc::now() + ChronoDuration::days(cadence_days as i64);
    sqlx::query!(
        r#"
        update playlist
        set last_updated_at = now(), next_update_at = $2
        where id = $1
        "#,
        playlist_id,
        next_update_at,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

async fn finalize_playlist_failure(
    pool: &PgPool,
    playlist_id: Uuid,
    deactivate: bool,
) -> Result<(), sqlx::Error> {
    // last_updated_at is left untouched on failure; next_update_at is
    // pulled forward for a retry sooner than the normal cadence. If the
    // playlist was deleted on Spotify's side (404), stop scheduling it.
    let next_update_at = Utc::now() + FAILURE_RETRY_DELAY;
    sqlx::query!(
        r#"
        update playlist
        set next_update_at = $2,
            is_active = case when $3 then false else is_active end
        where id = $1
        "#,
        playlist_id,
        next_update_at,
        deactivate,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_uris_additive_only_returns_new_tracks() {
        let current = vec!["a".to_string(), "b".to_string()];
        let new = vec!["b".to_string(), "c".to_string()];
        let (to_add, added, removed) = diff_uris(&current, &new);
        assert_eq!(to_add, vec!["c".to_string()]);
        assert_eq!(added, 1);
        assert_eq!(removed, 1);
    }

    #[test]
    fn diff_uris_no_overlap() {
        let current = vec!["a".to_string()];
        let new = vec!["b".to_string()];
        let (to_add, added, removed) = diff_uris(&current, &new);
        assert_eq!(to_add, vec!["b".to_string()]);
        assert_eq!(added, 1);
        assert_eq!(removed, 1);
    }

    #[test]
    fn diff_uris_identical_sets_add_nothing() {
        let current = vec!["a".to_string(), "b".to_string()];
        let new = vec!["b".to_string(), "a".to_string()];
        let (to_add, added, removed) = diff_uris(&current, &new);
        assert!(to_add.is_empty());
        assert_eq!(added, 0);
        assert_eq!(removed, 0);
    }

    #[test]
    fn diff_uris_empty_current_adds_everything() {
        let current: Vec<String> = vec![];
        let new = vec!["a".to_string(), "b".to_string()];
        let (to_add, added, removed) = diff_uris(&current, &new);
        assert_eq!(to_add.len(), 2);
        assert_eq!(added, 2);
        assert_eq!(removed, 0);
    }

    #[test]
    fn diff_uris_empty_new_removes_everything() {
        let current = vec!["a".to_string(), "b".to_string()];
        let new: Vec<String> = vec![];
        let (to_add, added, removed) = diff_uris(&current, &new);
        assert!(to_add.is_empty());
        assert_eq!(added, 0);
        assert_eq!(removed, 2);
    }
}
