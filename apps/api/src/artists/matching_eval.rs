//! Offline evaluation harness comparing candidate artist-matching
//! strategies against the real unmatched-artist corpus already sitting in
//! the live DB, before committing to any change in `worker.rs`.
//!
//! Read-only throughout: only ever reads `artists` and calls the real
//! Apple Music / Spotify catalog search APIs (no writes to `artists`, no
//! mutation of any kind). Needs a real `DATABASE_URL` plus working
//! Spotify/Apple Music credentials in the environment — the same ones
//! `build_app()` uses — so this is `#[ignore]`d and must be run
//! explicitly:
//!
//!   cargo test --lib artists::matching_eval -- --ignored --nocapture
//!
//! Strategies compared, for both Apple Music and Spotify (mirroring
//! `worker.rs`'s Apple-first-then-Spotify precedence):
//!   - `baseline`: today's actual logic — search top 1, require an
//!     exact-normalized name match.
//!   - `widened`: search the top `MAX_CANDIDATES` results, require an
//!     exact-normalized match against any of them (pure recall gain over
//!     baseline, zero precision cost — an exact match is still an exact
//!     match regardless of its rank in the search results).
//!   - `cleaned`: additionally tries `billing_variants(name)` (splitting
//!     obvious multi-artist billing, stripping common trailing
//!     qualifiers) against the widened search, in order, stopping at the
//!     first exact match.
//!
//! Output: a JSON report per strategy (recall against the real unmatched
//! sample, plus a regression check against a sample of already-matched
//! rows — does `widened`/`cleaned` ever land on a *different* artist than
//! what's already stored?) written to the scratchpad, and a summary
//! printed to stdout. Precision on any *newly found* match still needs a
//! human to spot-check the report — matching a cleaned string exactly
//! doesn't guarantee it's the right artist, just that it's not a fuzzy
//! guess.

use std::sync::Arc;

use reqwest::Client;
use serde::Serialize;
use sqlx::PgPool;

use crate::apple_music::auth::{AppleMusicCredentials, DeveloperTokenManager};
use crate::apple_music::client::AppleMusicClient;
use crate::auth::{ClientCredentialsManager, SpotifyCredentials};
use crate::db::AppDatabase;
use crate::spotify::client::{SpotifyClient, normalize_artist_name};

use super::cleaning::billing_variants;

/// Sample sizes default to a quick run (large enough to be a meaningful
/// signal, small enough to finish in a couple minutes) but are
/// overridable via env var for a full-corpus run:
///   EVAL_UNMATCHED_SAMPLE_SIZE=2000 EVAL_MATCHED_SAMPLE_SIZE=1500 \
///     cargo test --lib artists::matching_eval -- --ignored --nocapture
fn unmatched_sample_size() -> i64 {
    env_i64("EVAL_UNMATCHED_SAMPLE_SIZE", 150)
}
fn matched_sample_size() -> i64 {
    env_i64("EVAL_MATCHED_SAMPLE_SIZE", 60)
}
fn env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
const MAX_CANDIDATES: u8 = 10;
/// Same bounded-concurrency convention as `worker.rs`'s
/// `MAX_CONCURRENT_ENRICHMENTS` -- a polite ceiling on concurrent
/// requests against either provider.
const MAX_CONCURRENT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
enum Strategy {
    Baseline,
    Widened,
    Cleaned,
}

impl Strategy {
    fn label(self) -> &'static str {
        match self {
            Strategy::Baseline => "baseline",
            Strategy::Widened => "widened",
            Strategy::Cleaned => "cleaned",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct MatchOutcome {
    original_name: String,
    strategy: &'static str,
    provider: Option<&'static str>,
    matched: bool,
    matched_variant: Option<String>,
    matched_name: Option<String>,
    matched_id: Option<String>,
    /// Only set for the regression pass -- the id already stored in
    /// `artists` for this name, to compare against `matched_id`.
    previously_matched_id: Option<String>,
}

struct Providers {
    apple: AppleMusicClient,
    apple_dev_token: String,
    spotify: SpotifyClient,
    spotify_token: String,
}

async fn build_providers() -> Providers {
    let http = Client::new();

    let apple_creds = AppleMusicCredentials::from_env();
    let storefront = std::env::var("APPLE_MUSIC_STOREFRONT").unwrap_or_else(|_| "us".into());
    let apple = AppleMusicClient::new(http.clone(), storefront);
    let dev_mgr = DeveloperTokenManager::new(apple_creds);
    let apple_dev_token = dev_mgr
        .get_token()
        .await
        .expect("failed to generate Apple Music developer token -- check APPLE_MUSIC_* env vars");

    let spotify_creds = SpotifyCredentials::from_env();
    let cc_mgr = ClientCredentialsManager::new(spotify_creds, http.clone());
    let spotify_token = cc_mgr
        .get_token()
        .await
        .expect("failed to get Spotify client-credentials token -- check SPOTIFY_CLIENT_* env vars")
        .access_token;
    let spotify = SpotifyClient::new(http);

    Providers {
        apple,
        apple_dev_token,
        spotify,
        spotify_token,
    }
}

async fn apple_candidates(providers: &Providers, term: &str, limit: u8) -> Vec<(String, String)> {
    providers
        .apple
        .search_artists(&providers.apple_dev_token, term, limit)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter_map(|a| a.attributes.map(|attrs| (a.id.clone(), attrs.name)))
        .collect()
}

async fn spotify_candidates(providers: &Providers, term: &str, limit: u8) -> Vec<(String, String)> {
    providers
        .spotify
        .search_artist(&providers.spotify_token, term, limit)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id, a.name))
        .collect()
}

/// Runs one strategy against one name for one provider. Returns `None`
/// when the strategy simply doesn't apply variants beyond what an earlier
/// strategy already covers (used to skip redundant `baseline` work when
/// `widened`/`cleaned` are supersets).
async fn run_strategy(
    providers: &Providers,
    provider: &'static str,
    strategy: Strategy,
    original_name: &str,
) -> Option<(String, String, String)> {
    let variants: Vec<String> = match strategy {
        Strategy::Baseline | Strategy::Widened => vec![original_name.to_string()],
        Strategy::Cleaned => {
            let mut v = vec![original_name.to_string()];
            v.extend(billing_variants(original_name));
            v
        }
    };
    let limit = match strategy {
        Strategy::Baseline => 1,
        Strategy::Widened | Strategy::Cleaned => MAX_CANDIDATES,
    };

    for variant in variants {
        // Compare against *this variant's* normalized form, not the
        // original name's -- a cleaned variant ("Zedd") is supposed to
        // match a candidate literally named "Zedd", which will never
        // equal the normalized form of the full uncleaned billing string.
        let variant_normalized = normalize_artist_name(&variant);
        let candidates = match provider {
            "apple_music" => apple_candidates(providers, &variant, limit).await,
            "spotify" => spotify_candidates(providers, &variant, limit).await,
            _ => unreachable!(),
        };
        if let Some((id, name)) = candidates
            .into_iter()
            .find(|(_, name)| normalize_artist_name(name) == variant_normalized)
        {
            return Some((variant, id, name));
        }
    }
    None
}

async fn evaluate_name(
    providers: &Providers,
    strategy: Strategy,
    original_name: String,
    previously_matched_id: Option<String>,
) -> MatchOutcome {
    // Apple first, Spotify as fallback -- same precedence as worker.rs.
    if let Some((variant, id, name)) =
        run_strategy(providers, "apple_music", strategy, &original_name).await
    {
        return MatchOutcome {
            original_name,
            strategy: strategy.label(),
            provider: Some("apple_music"),
            matched: true,
            matched_variant: Some(variant),
            matched_name: Some(name),
            matched_id: Some(id),
            previously_matched_id,
        };
    }
    if let Some((variant, id, name)) =
        run_strategy(providers, "spotify", strategy, &original_name).await
    {
        return MatchOutcome {
            original_name,
            strategy: strategy.label(),
            provider: Some("spotify"),
            matched: true,
            matched_variant: Some(variant),
            matched_name: Some(name),
            matched_id: Some(id),
            previously_matched_id,
        };
    }

    MatchOutcome {
        original_name,
        strategy: strategy.label(),
        provider: None,
        matched: false,
        matched_variant: None,
        matched_name: None,
        matched_id: None,
        previously_matched_id,
    }
}

async fn evaluate_batch(
    providers: Arc<Providers>,
    strategy: Strategy,
    names: Vec<(String, Option<String>)>,
) -> Vec<MatchOutcome> {
    use futures::StreamExt;

    stream_names(names, providers, strategy).collect().await
}

fn stream_names(
    names: Vec<(String, Option<String>)>,
    providers: Arc<Providers>,
    strategy: Strategy,
) -> impl futures::Stream<Item = MatchOutcome> {
    use futures::StreamExt;

    futures::stream::iter(names)
        .map(move |(name, prev_id)| {
            let providers = providers.clone();
            async move { evaluate_name(&providers, strategy, name, prev_id).await }
        })
        .buffer_unordered(MAX_CONCURRENT)
}

async fn fetch_unmatched_sample(pool: &PgPool) -> Vec<String> {
    let limit = unmatched_sample_size();
    sqlx::query_scalar!(
        r#"
        select display_name
        from artists
        where matched_via is null
        order by random()
        limit $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
    .expect("failed to fetch unmatched artist sample")
}

struct MatchedRow {
    display_name: String,
    stored_id: Option<String>,
}

async fn fetch_matched_sample(pool: &PgPool) -> Vec<MatchedRow> {
    let limit = matched_sample_size();
    sqlx::query!(
        r#"
        select
            display_name,
            coalesce(apple_music_id, spotify_id) as "stored_id"
        from artists
        where matched_via is not null
        order by random()
        limit $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await
    .expect("failed to fetch matched artist sample")
    .into_iter()
    .map(|r| MatchedRow {
        display_name: r.display_name,
        stored_id: r.stored_id,
    })
    .collect()
}

#[derive(Serialize)]
struct Report {
    unmatched_sample_size: usize,
    matched_sample_size: usize,
    recall: Vec<StrategySummary>,
    regressions: Vec<MatchOutcome>,
    unmatched_outcomes: Vec<MatchOutcome>,
}

#[derive(Serialize)]
struct StrategySummary {
    strategy: &'static str,
    /// Out of the unmatched sample, how many this strategy newly matched.
    new_matches: usize,
    sample_size: usize,
}

#[tokio::test]
#[ignore]
async fn eval_matching_strategies() {
    let _ = dotenvy::dotenv();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run this eval");
    let db = AppDatabase::new(&database_url)
        .await
        .expect("failed to connect to database");

    let providers = Arc::new(build_providers().await);

    let unmatched = fetch_unmatched_sample(&db.pool).await;
    let matched = fetch_matched_sample(&db.pool).await;
    println!(
        "loaded {} unmatched / {} matched names to evaluate",
        unmatched.len(),
        matched.len()
    );

    let mut recall = Vec::new();
    let mut unmatched_outcomes = Vec::new();

    for strategy in [Strategy::Baseline, Strategy::Widened, Strategy::Cleaned] {
        let names: Vec<(String, Option<String>)> =
            unmatched.iter().cloned().map(|n| (n, None)).collect();
        let sample_size = names.len();
        let outcomes = evaluate_batch(providers.clone(), strategy, names).await;
        let new_matches = outcomes.iter().filter(|o| o.matched).count();
        println!(
            "[{}] {}/{} of the unmatched sample now match",
            strategy.label(),
            new_matches,
            sample_size
        );
        recall.push(StrategySummary {
            strategy: strategy.label(),
            new_matches,
            sample_size,
        });
        unmatched_outcomes.extend(outcomes);
    }

    // Regression pass: does `widened`/`cleaned` ever land on a *different*
    // artist than what's already trusted for a name that already matches
    // today? `baseline` is skipped here -- it's definitionally identical
    // to what produced `stored_id` in the first place.
    let mut regressions = Vec::new();
    for strategy in [Strategy::Widened, Strategy::Cleaned] {
        let names: Vec<(String, Option<String>)> = matched
            .iter()
            .map(|r| (r.display_name.clone(), r.stored_id.clone()))
            .collect();
        let outcomes = evaluate_batch(providers.clone(), strategy, names).await;
        let flipped: Vec<MatchOutcome> = outcomes
            .into_iter()
            .filter(|o| {
                o.matched
                    && o.previously_matched_id.is_some()
                    && o.matched_id != o.previously_matched_id
            })
            .collect();
        if !flipped.is_empty() {
            println!(
                "[{}] {} previously-matched names flipped to a different id -- see report",
                strategy.label(),
                flipped.len()
            );
        }
        regressions.extend(flipped);
    }

    let report = Report {
        unmatched_sample_size: unmatched.len(),
        matched_sample_size: matched.len(),
        recall,
        regressions,
        unmatched_outcomes,
    };

    let out_path = std::env::var("EVAL_OUTPUT_PATH")
        .unwrap_or_else(|_| "/tmp/gigeo_matching_eval_report.json".to_string());
    std::fs::write(
        &out_path,
        serde_json::to_string_pretty(&report).expect("failed to serialize report"),
    )
    .expect("failed to write report");
    println!("full report written to {out_path}");
}
