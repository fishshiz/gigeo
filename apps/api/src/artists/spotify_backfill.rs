//! One-off backfill: attaches a Spotify identity to artists that were
//! matched via Apple Music before `worker::resolve_fresh` started looking
//! Spotify up alongside Apple Music too. Without this, those rows would
//! only pick up a Spotify listen link whenever their dynamic-field refresh
//! cycle happens to fire next, up to 30 days out (see
//! `worker::DYNAMIC_FIELD_TTL`).
//!
//! Writes real data to `artists` (via `db::attach_spotify`, which never
//! overwrites an existing `spotify_id`) using the real Spotify API, so like
//! `matching_eval` this needs a real `DATABASE_URL` plus working Spotify
//! credentials and is `#[ignore]`d, run explicitly:
//!
//!   cargo test --lib artists::spotify_backfill -- --ignored --nocapture
//!
//! Safe to interrupt and re-run at any point: the selection query only
//! pulls rows still missing `spotify_url`, and `attach_spotify`'s own
//! `where spotify_id is null` guard makes even an overlapping write a
//! no-op — no separate progress-tracking needed.

use futures::StreamExt;
use reqwest::Client;
use sqlx::PgPool;

use super::db;
use super::worker::try_spotify_match;
use crate::auth::{ClientCredentialsManager, SpotifyCredentials};
use crate::db::AppDatabase;
use crate::spotify::client::SpotifyClient;

/// Caps the number of candidate rows processed in one run — unset means
/// the full backlog. Useful for a small sanity-check run before committing
/// to the whole backlog:
///   BACKFILL_LIMIT=25 cargo test --lib artists::spotify_backfill -- --ignored --nocapture
fn backfill_limit() -> i64 {
    std::env::var("BACKFILL_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(i64::MAX)
}

/// Same bounded-concurrency convention as
/// `worker::MAX_CONCURRENT_ENRICHMENTS` — a polite ceiling on concurrent
/// requests against Spotify.
const MAX_CONCURRENT: usize = 4;

struct BackfillTarget {
    normalized_name: String,
    display_name: String,
}

async fn fetch_targets(pool: &PgPool, limit: i64) -> Vec<BackfillTarget> {
    sqlx::query_as!(
        BackfillTarget,
        r#"
        select normalized_name, display_name
        from artists
        where matched_via::text = 'apple_music' and spotify_url is null
        order by normalized_name
        limit $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
    .expect("failed to fetch backfill targets")
}

enum BackfillOutcome {
    Found { name: String },
    NotFound { name: String },
    DbError { name: String, error: String },
}

async fn backfill_one(
    spotify: &SpotifyClient,
    cc_manager: &ClientCredentialsManager,
    pool: &PgPool,
    target: BackfillTarget,
) -> BackfillOutcome {
    let Some(artist) = try_spotify_match(spotify, cc_manager, &target.display_name).await else {
        return BackfillOutcome::NotFound {
            name: target.display_name,
        };
    };

    let result = db::attach_spotify(
        pool,
        &target.normalized_name,
        &artist.id,
        artist.external_urls.spotify.as_deref(),
    )
    .await;

    match result {
        Ok(()) => BackfillOutcome::Found {
            name: target.display_name,
        },
        Err(err) => BackfillOutcome::DbError {
            name: target.display_name,
            error: err.to_string(),
        },
    }
}

#[tokio::test]
#[ignore]
async fn backfill_spotify_for_apple_matched_artists() {
    let _ = dotenvy::dotenv();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run this backfill");
    let db = AppDatabase::new(&database_url)
        .await
        .expect("failed to connect to database");

    let http = Client::new();
    let spotify_creds = SpotifyCredentials::from_env();
    let cc_manager = ClientCredentialsManager::new(spotify_creds, http.clone());
    let spotify = SpotifyClient::new(http);

    let targets = fetch_targets(&db.pool, backfill_limit()).await;
    let total = targets.len();
    println!("backfilling spotify identity for {total} apple_music-matched artists");

    let outcomes: Vec<BackfillOutcome> = futures::stream::iter(targets)
        .map(|target| async { backfill_one(&spotify, &cc_manager, &db.pool, target).await })
        .buffer_unordered(MAX_CONCURRENT)
        .collect()
        .await;

    let mut found = Vec::new();
    let mut not_found = Vec::new();
    let mut errors = Vec::new();
    for outcome in outcomes {
        match outcome {
            BackfillOutcome::Found { name } => found.push(name),
            BackfillOutcome::NotFound { name } => not_found.push(name),
            BackfillOutcome::DbError { name, error } => errors.push((name, error)),
        }
    }

    println!(
        "attempted={total} found={} not_found={} errors={}",
        found.len(),
        not_found.len(),
        errors.len()
    );
    if !not_found.is_empty() {
        println!("not found: {not_found:?}");
    }
    if !errors.is_empty() {
        println!("errors: {errors:?}");
    }
}
