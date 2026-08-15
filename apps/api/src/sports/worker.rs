//! Resolves a Ticketmaster team performer to an ESPN team identity, and
//! keeps its current record/standing fresh -- the write path behind both
//! `spawn_enrichment` (fired from the live request stream, mirroring
//! `crate::artists::worker`) and the on-demand `/sports/enrichment`
//! endpoint (`super::get_team_enrichment`).
//!
//! No batch job here either, for the same reason as `artists`: there's no
//! persisted `events`/`performers` table to batch over (ADR-0001).
//!
//! Simpler than `artists::worker` in one respect: ESPN's `/standings`
//! call returns every team in a league at once, so a single fetch both
//! matches a brand-new team *and* warms the standings cache for every
//! other team in that league -- there's no separate "search for one
//! team" step the way `artists` needs Apple Music/Spotify catalog search.

use std::collections::HashMap;

use chrono::{Duration, Utc};
use futures::{StreamExt, stream};

use super::client;
use super::db;
use super::types::SportsTeamCandidate;
use crate::spotify::client::normalize_artist_name;
use crate::state::AppState;

/// Caps concurrent ESPN lookups for the same reason
/// `artists::worker::MAX_CONCURRENT_ENRICHMENTS` does -- a dense
/// request's worth of lookups fired at once is just needless load on an
/// unofficial endpoint with no published rate limit to lean on.
const MAX_CONCURRENT_ENRICHMENTS: usize = 4;

/// How long to wait before retrying a team name that previously found no
/// confident match. Mirrors `artists::worker::UNMATCHED_RETRY_COOLDOWN` --
/// team rosters/names don't change often, so there's no need to retry
/// aggressively.
const UNMATCHED_RETRY_COOLDOWN: Duration = Duration::days(7);

/// How long a resolved team's record/standing is trusted before being
/// re-fetched. Team *identity* has no equivalent TTL (see module docs) --
/// only this dynamic field does.
const STANDINGS_TTL: Duration = Duration::days(1);

/// Fires off best-effort background matching for `candidates` without
/// blocking the caller -- see module docs on why this, not a batch job,
/// triggers enrichment.
pub(crate) fn spawn_enrichment(state: &AppState, candidates: Vec<SportsTeamCandidate>) {
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

/// Dedupes by Ticketmaster attraction id -- the cache key
/// (`sports_team_matches.ticketmaster_attraction_id`), so a request with
/// several games featuring the same team doesn't queue the same lookup
/// repeatedly.
fn dedupe_candidates(candidates: Vec<SportsTeamCandidate>) -> Vec<SportsTeamCandidate> {
    let mut by_id: HashMap<String, SportsTeamCandidate> = HashMap::new();
    for candidate in candidates {
        if let Some(id) = candidate.ticketmaster_attraction_id.clone() {
            by_id.entry(id).or_insert(candidate);
        }
    }
    by_id.into_values().collect()
}

/// Resolves (from cache, or freshly) `candidate`'s ESPN team id, then
/// ensures its standings are fresh. A no-op for a candidate with no
/// Ticketmaster attraction id -- there's nothing to key the cache on (see
/// `dedupe_candidates`).
pub(super) async fn resolve_one(state: &AppState, candidate: SportsTeamCandidate) {
    let Some(tm_id) = candidate.ticketmaster_attraction_id.clone() else {
        return;
    };

    let existing = match db::get_match(&state.db.pool, &tm_id).await {
        Ok(row) => row,
        Err(err) => {
            tracing::warn!(error = %err, name = %candidate.name, "sports enrichment: failed to read existing match, skipping");
            return;
        }
    };

    if let Some(row) = &existing {
        if let Some(team_id) = &row.espn_team_id {
            let fresh = match db::get_standings(&state.db.pool, candidate.league, team_id).await {
                Ok(Some(standing)) => Utc::now() - standing.fetched_at < STANDINGS_TTL,
                _ => false,
            };
            if fresh {
                return;
            }
        } else if Utc::now() - row.last_attempted_at < UNMATCHED_RETRY_COOLDOWN {
            return;
        }
    }

    // Either unmatched (first sighting, or a cooled-down retry) or
    // matched-but-stale -- either way, a fresh league-wide fetch both
    // resolves the match (if needed) and refreshes standings, since ESPN
    // returns every team in one call.
    let teams = match client::get_standings(&state.client, candidate.league).await {
        Ok(teams) => teams,
        Err(err) => {
            tracing::warn!(error = %err, "sports enrichment: failed to fetch league standings");
            return;
        }
    };

    for team in &teams {
        if let Err(err) = db::upsert_standings(
            &state.db.pool,
            db::NewStanding {
                league: candidate.league,
                espn_team_id: &team.team_id,
                record: team.record.as_deref().unwrap_or(""),
                standing_position: team.standing_position,
                group_name: Some(&team.group_name),
            },
        )
        .await
        {
            tracing::warn!(error = %err, "sports enrichment: failed to persist standings");
        }
    }

    // Already matched -- standings for every team in the league (this one
    // included) were just refreshed above, so there's nothing left to
    // resolve.
    if existing.is_some_and(|row| row.espn_team_id.is_some()) {
        return;
    }

    match_team(state, &candidate, &tm_id, &teams).await;
}

/// Searches the just-fetched league standings for `candidate.name` and
/// persists the result -- a matched team id on an exact-normalized name
/// match (the same verified-match rule `artists::worker` uses for artist
/// names, not fuzzy/scored matching), or a tombstone on no match.
async fn match_team(
    state: &AppState,
    candidate: &SportsTeamCandidate,
    tm_id: &str,
    teams: &[client::TeamStanding],
) {
    let normalized = normalize_artist_name(&candidate.name);
    if normalized.is_empty() {
        return;
    }

    let found = teams
        .iter()
        .find(|t| normalize_artist_name(&t.team_name) == normalized);

    match found {
        Some(team) => {
            if let Err(err) = db::upsert_match_found(
                &state.db.pool,
                tm_id,
                candidate.league,
                &team.team_id,
                &team.team_name,
            )
            .await
            {
                tracing::warn!(error = %err, name = %candidate.name, "sports enrichment: failed to persist team match");
            }
        }
        None => {
            if let Err(err) =
                db::upsert_match_tombstone(&state.db.pool, tm_id, candidate.league).await
            {
                tracing::warn!(error = %err, name = %candidate.name, "sports enrichment: failed to persist no-match tombstone");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sports::types::League;

    #[test]
    fn dedupe_candidates_collapses_repeats_of_the_same_attraction() {
        let candidates = vec![
            SportsTeamCandidate {
                name: "Lakers".into(),
                ticketmaster_attraction_id: Some("a1".into()),
                league: League::Nba,
            },
            SportsTeamCandidate {
                name: "Los Angeles Lakers".into(),
                ticketmaster_attraction_id: Some("a1".into()),
                league: League::Nba,
            },
        ];

        assert_eq!(dedupe_candidates(candidates).len(), 1);
    }

    #[test]
    fn dedupe_candidates_drops_candidates_with_no_attraction_id() {
        let candidates = vec![SportsTeamCandidate {
            name: "Lakers".into(),
            ticketmaster_attraction_id: None,
            league: League::Nba,
        }];

        assert!(dedupe_candidates(candidates).is_empty());
    }
}
