//! Sports (standings/record) enrichment for major-league matchup events --
//! see `types::matchups_from` for what counts as eligible, and
//! `migrations/007_sports_enrichment.sql` for why this is two cache
//! tables rather than one. Data source is ESPN's free public standings
//! API (see `client` module docs on why, over a paid provider).
//!
//! Mirrors `crate::artists`' shape: `spawn_enrichment` is called from the
//! live Ticketmaster request path (`events::stream::get_concerts_tm_stream`)
//! so the cache is often already warm by the time a user opens an event's
//! detail view; the on-demand `/sports/enrichment` endpoint below is what
//! that detail view actually calls, resolving (or refreshing a stale
//! standing for) one team right now if it isn't warm yet.
//!
//! Unlike `artists`, there's no inline list-stream attachment -- this
//! feature is detail-view-only by design (a standings line doesn't fit an
//! event card), so nothing here touches `EventResponse` itself.

mod client;
mod db;
pub(crate) mod types;
mod worker;

pub(crate) use types::matchups_from;
pub(crate) use worker::spawn_enrichment;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use types::{League, SportsTeamCandidate};

#[derive(Deserialize)]
pub struct TeamEnrichmentQuery {
    pub name: String,
    #[serde(rename = "ticketmasterAttractionId")]
    pub ticketmaster_attraction_id: Option<String>,
    /// The subGenre value straight off the event payload ("NBA", "NFL",
    /// ...) -- matched case-insensitively against `League::from_db_str`
    /// so the frontend doesn't need its own copy of this codebase's
    /// internal league naming.
    pub league: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct TeamEnrichmentResponse {
    #[serde(rename = "teamName")]
    pub team_name: String,
    /// ESPN's own formatted record summary -- see `client::TeamStanding`
    /// doc comment on why this isn't split into wins/losses columns.
    pub record: String,
    #[serde(rename = "groupName")]
    pub group_name: Option<String>,
    /// The full conference/division table (every cached team sharing
    /// `group_name`), ordered by standing -- a bare position number on
    /// its own doesn't say much without the teams around it. Empty when
    /// `group_name` is absent, or nothing else in the group has been
    /// cached yet.
    pub standings: Vec<StandingEntry>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct StandingEntry {
    #[serde(rename = "teamName")]
    pub team_name: String,
    pub record: String,
    #[serde(rename = "standingPosition")]
    pub standing_position: Option<i32>,
}

/// `GET /sports/enrichment?name=...&ticketmasterAttractionId=...&league=NBA`
/// -- resolves and returns one team's current record/standing right now (a
/// cheap cache read for an already-matched team; a real ESPN lookup, same
/// as any other first sighting, for a brand-new one). Meant to be called
/// once per team on a selected matchup's detail view (two calls for a
/// matchup), mirroring `artists::get_performer_enrichment`. `None` covers
/// "no confident match", "not one of the four major leagues", and any DB/
/// provider error along the way -- best-effort, same as `artists`.
pub async fn get_team_enrichment(
    State(state): State<AppState>,
    Query(query): Query<TeamEnrichmentQuery>,
) -> Json<Option<TeamEnrichmentResponse>> {
    let Some(league) = League::from_db_str(&query.league.to_lowercase()) else {
        return Json(None);
    };
    let Some(tm_id) = query.ticketmaster_attraction_id.clone() else {
        return Json(None);
    };

    worker::resolve_one(
        &state,
        SportsTeamCandidate {
            name: query.name,
            ticketmaster_attraction_id: Some(tm_id.clone()),
            league,
        },
    )
    .await;

    Json(lookup(&state, league, &tm_id).await)
}

async fn lookup(state: &AppState, league: League, tm_id: &str) -> Option<TeamEnrichmentResponse> {
    let matched = db::get_match(&state.db.pool, tm_id).await.ok().flatten()?;
    let team_id = matched.espn_team_id?;
    let standings = db::get_standings(&state.db.pool, league, &team_id)
        .await
        .ok()
        .flatten()?;

    let group_standings = match &standings.group_name {
        Some(group_name) => db::get_group_standings(&state.db.pool, league, group_name)
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };

    Some(TeamEnrichmentResponse {
        team_name: matched.resolved_name.unwrap_or_default(),
        record: standings.record,
        group_name: standings.group_name,
        standings: group_standings
            .into_iter()
            .map(|row| StandingEntry {
                team_name: row.team_name,
                record: row.record,
                standing_position: row.standing_position,
            })
            .collect(),
    })
}
