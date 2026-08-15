//! Persistence for `sports_team_matches` (Ticketmaster attraction -> ESPN
//! team identity, treated as permanent once resolved -- same posture as
//! `artists.matched_via`, see ADR-0001) and `sports_team_standings`
//! (current record/standing, TTL-refreshed). See
//! `migrations/007_sports_enrichment.sql` for the schema and why these
//! are two tables rather than one.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::types::League;

pub(super) struct MatchRow {
    pub espn_team_id: Option<String>,
    pub resolved_name: Option<String>,
    pub last_attempted_at: DateTime<Utc>,
}

pub(super) async fn get_match(
    pool: &PgPool,
    ticketmaster_attraction_id: &str,
) -> Result<Option<MatchRow>, sqlx::Error> {
    sqlx::query_as!(
        MatchRow,
        r#"
        select espn_team_id, resolved_name, last_attempted_at
        from sports_team_matches
        where ticketmaster_attraction_id = $1
        "#,
        ticketmaster_attraction_id
    )
    .fetch_optional(pool)
    .await
}

/// Persists a confirmed team match. `on conflict` always overwrites --
/// unlike `artists::db::upsert_matched`'s coalesce-on-identity-fields
/// dance, there's no equivalent "another normalized name already claimed
/// this identity" case here: `ticketmaster_attraction_id` is the primary
/// key, so a conflict can only mean this exact attraction was already
/// resolved (possibly to a different team, if Ticketmaster's data ever
/// changes under it), and the fresh result should simply win.
pub(super) async fn upsert_match_found(
    pool: &PgPool,
    ticketmaster_attraction_id: &str,
    league: League,
    espn_team_id: &str,
    resolved_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into sports_team_matches (ticketmaster_attraction_id, league, espn_team_id, resolved_name, last_attempted_at)
        values ($1, $2::text::sports_league, $3, $4, now())
        on conflict (ticketmaster_attraction_id) do update set
            league = excluded.league,
            espn_team_id = excluded.espn_team_id,
            resolved_name = excluded.resolved_name,
            last_attempted_at = now()
        "#,
        ticketmaster_attraction_id,
        league.as_db_str(),
        espn_team_id,
        resolved_name,
    )
    .execute(pool)
    .await
    .map(|_| ())
}

/// Records a no-confident-match attempt, so a name that will never
/// resolve (a typo, a non-team attraction Ticketmaster still tagged with
/// a major-league subGenre) isn't re-searched on every request that
/// includes it -- see `worker::UNMATCHED_RETRY_COOLDOWN` for when it's
/// retried.
pub(super) async fn upsert_match_tombstone(
    pool: &PgPool,
    ticketmaster_attraction_id: &str,
    league: League,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into sports_team_matches (ticketmaster_attraction_id, league, last_attempted_at)
        values ($1, $2::text::sports_league, now())
        on conflict (ticketmaster_attraction_id) do update set
            last_attempted_at = now()
        "#,
        ticketmaster_attraction_id,
        league.as_db_str(),
    )
    .execute(pool)
    .await
    .map(|_| ())
}

pub(super) struct StandingsRow {
    pub record: String,
    pub standing_position: Option<i32>,
    pub group_name: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

pub(super) async fn get_standings(
    pool: &PgPool,
    league: League,
    espn_team_id: &str,
) -> Result<Option<StandingsRow>, sqlx::Error> {
    sqlx::query_as!(
        StandingsRow,
        r#"
        select record, standing_position, group_name, fetched_at
        from sports_team_standings
        where league = $1::text::sports_league and espn_team_id = $2
        "#,
        league.as_db_str(),
        espn_team_id,
    )
    .fetch_optional(pool)
    .await
}

/// Upserts every team's current record/standing in `teams` in one
/// round-trip (`unnest` over parallel arrays), rather than one query per
/// team -- no history kept, nothing today reads a past standing, only
/// "as of now" (see migration docs).
///
/// Not just an optimization: confirmed live that a plain per-team loop
/// takes ~33 seconds for a league the size of Division I men's college
/// basketball (~350 teams, sequential round-trips to a remote Postgres
/// instance) -- long enough to make the on-demand detail-view endpoint
/// this feeds feel broken. A no-op for an empty slice (nothing to
/// `unnest`).
pub(super) async fn upsert_standings_bulk(
    pool: &PgPool,
    league: League,
    teams: &[super::client::TeamStanding],
) -> Result<(), sqlx::Error> {
    if teams.is_empty() {
        return Ok(());
    }

    let espn_team_ids: Vec<&str> = teams.iter().map(|t| t.team_id.as_str()).collect();
    let records: Vec<&str> = teams
        .iter()
        .map(|t| t.record.as_deref().unwrap_or(""))
        .collect();
    let standing_positions: Vec<Option<i32>> = teams.iter().map(|t| t.standing_position).collect();
    let group_names: Vec<&str> = teams.iter().map(|t| t.group_name.as_str()).collect();

    sqlx::query!(
        r#"
        insert into sports_team_standings (league, espn_team_id, record, standing_position, group_name, fetched_at)
        select $1::text::sports_league, u.espn_team_id, u.record, u.standing_position, u.group_name, now()
        from unnest($2::text[], $3::text[], $4::integer[], $5::text[])
            as u(espn_team_id, record, standing_position, group_name)
        on conflict (league, espn_team_id) do update set
            record = excluded.record,
            standing_position = excluded.standing_position,
            group_name = excluded.group_name,
            fetched_at = now()
        "#,
        league.as_db_str(),
        &espn_team_ids as &[&str],
        &records as &[&str],
        &standing_positions as &[Option<i32>],
        &group_names as &[&str],
    )
    .execute(pool)
    .await
    .map(|_| ())
}
