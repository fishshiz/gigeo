//! HTTP client for ESPN's public standings API -- the same undocumented
//! endpoint that powers espn.com's own standings pages. Chosen over
//! API-SPORTS after live testing showed API-SPORTS' free tier gates
//! current-season data behind a paid plan (2022-2024 only, confirmed
//! live for two of the four products) -- not worth it for a portfolio
//! project when this needs no key at all and already returns current
//! data.
//!
//! One host and one endpoint shape covers all four leagues (unlike
//! API-SPORTS' four separate products with genuinely different response
//! shapes) -- confirmed live for NBA/NFL/NHL/MLB during this feature's
//! design. A single `/standings` call per league returns every team's
//! record, playoff seed, and conference in one shot, so there's no
//! separate team-search step: matching (`worker::match_team`) searches
//! this same response instead of a dedicated endpoint.
//!
//! This is unofficial -- no contract, no SLA, and it could change shape
//! or disappear without notice. Acceptable for a portfolio project; a
//! commercial one should budget for a supported provider instead.
//!
//! One real gotcha, confirmed live: ESPN's WAF 403s requests carrying no
//! `User-Agent` header or a generic HTTP-library one (reqwest's own
//! default, browser-impersonating strings) while allowing well-formed
//! self-identifying ones -- so every request here sends `ESPN_USER_AGENT`
//! rather than reqwest's default.

use reqwest::Client;
use serde::Deserialize;

use super::types::League;
use crate::error::AppError;
use crate::http_utils::request_with_backoff;

/// See module docs on why this is set explicitly -- a request with no
/// `User-Agent` (reqwest's default) gets 403'd by ESPN's WAF, confirmed
/// live. The `(+url)` form follows standard well-behaved-bot convention
/// (as `Googlebot/2.1 (+http://www.google.com/bot.html)` does) rather
/// than impersonating another tool.
const ESPN_USER_AGENT: &str = "gigeo/1.0 (+https://github.com/fishshiz/gigeo)";

#[derive(Debug, Deserialize)]
struct StandingsResponse {
    children: Vec<ConferenceGroup>,
}

#[derive(Debug, Deserialize)]
struct ConferenceGroup {
    name: String,
    standings: EntriesWrapper,
}

#[derive(Debug, Deserialize)]
struct EntriesWrapper {
    entries: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    team: EntryTeam,
    stats: Vec<Stat>,
}

#[derive(Debug, Deserialize)]
struct EntryTeam {
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct Stat {
    #[serde(rename = "type")]
    stat_type: String,
    summary: Option<String>,
    #[serde(rename = "displayValue")]
    display_value: Option<String>,
}

/// One team's current standing, flattened out of ESPN's nested
/// conference/entries/stats response into just what this feature needs.
#[derive(Debug, Clone)]
pub(crate) struct TeamStanding {
    pub team_id: String,
    pub team_name: String,
    pub group_name: String,
    /// ESPN's own formatted record summary (the `stats[type="total"]`
    /// entry) -- e.g. `"60-22"`, or `"53-22-7"` for a hockey team's
    /// wins-losses-OT-losses. Kept as ESPN formats it rather than split
    /// into columns, since the shape of "a record" varies by sport (see
    /// migration docs).
    pub record: Option<String>,
    /// Playoff seed within `group_name` -- the closest equivalent to a
    /// flat standing position ESPN exposes; confirmed present under the
    /// same `"playoffseed"` stat type across all four sports.
    pub standing_position: Option<i32>,
}

/// `GET /apis/v2/sports/{sport}/{league}/standings` -- confirmed live for
/// all four leagues. No auth, no season/league-id parameters needed:
/// this always returns "now".
pub(crate) async fn get_standings(
    client: &Client,
    league: League,
) -> Result<Vec<TeamStanding>, AppError> {
    let path = league.espn_path();
    let url = format!("https://site.api.espn.com/apis/v2/sports/{path}/standings");

    let resp = request_with_backoff(
        "espn",
        || client.get(&url).header("User-Agent", ESPN_USER_AGENT),
        |status, message| AppError::SportsApi { status, message },
    )
    .await?;

    let parsed: StandingsResponse = resp.json().await?;

    Ok(parsed
        .children
        .into_iter()
        .flat_map(|group| {
            let group_name = group.name;
            group
                .standings
                .entries
                .into_iter()
                .map(move |entry| to_team_standing(&group_name, entry))
        })
        .collect())
}

fn to_team_standing(group_name: &str, entry: Entry) -> TeamStanding {
    let record = entry
        .stats
        .iter()
        .find(|s| s.stat_type == "total")
        .and_then(|s| s.summary.clone());
    let standing_position = entry
        .stats
        .iter()
        .find(|s| s.stat_type == "playoffseed")
        .and_then(|s| s.display_value.as_deref())
        .and_then(|v| v.parse().ok());

    TeamStanding {
        team_id: entry.team.id,
        team_name: entry.team.display_name,
        group_name: group_name.to_string(),
        record,
        standing_position,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trimmed real response, captured live from
    /// `GET /apis/v2/sports/basketball/nba/standings` during this
    /// feature's design (one conference, one team, extraneous fields --
    /// logos, links, unrelated stats -- stripped, but every field this
    /// module actually reads is verbatim). Guards against the deserialize
    /// silently going empty on a field-name mismatch, which
    /// `#[derive(Deserialize)]` won't catch at compile time.
    const REAL_NBA_FIXTURE: &str = r#"{
        "children": [
            {
                "name": "Eastern Conference",
                "standings": {
                    "entries": [
                        {
                            "team": { "id": "8", "displayName": "Detroit Pistons" },
                            "stats": [
                                {
                                    "type": "total",
                                    "name": "overall",
                                    "summary": "60-22",
                                    "displayValue": "60-22"
                                },
                                {
                                    "type": "playoffseed",
                                    "name": "playoffSeed",
                                    "displayValue": "1"
                                }
                            ]
                        }
                    ]
                }
            }
        ]
    }"#;

    #[test]
    fn parses_a_real_captured_response_into_the_expected_team_standing() {
        let parsed: StandingsResponse = serde_json::from_str(REAL_NBA_FIXTURE).unwrap();
        let standings: Vec<TeamStanding> = parsed
            .children
            .into_iter()
            .flat_map(|group| {
                let group_name = group.name;
                group
                    .standings
                    .entries
                    .into_iter()
                    .map(move |entry| to_team_standing(&group_name, entry))
            })
            .collect();

        assert_eq!(standings.len(), 1);
        let pistons = &standings[0];
        assert_eq!(pistons.team_id, "8");
        assert_eq!(pistons.team_name, "Detroit Pistons");
        assert_eq!(pistons.group_name, "Eastern Conference");
        assert_eq!(pistons.record.as_deref(), Some("60-22"));
        assert_eq!(pistons.standing_position, Some(1));
    }

    #[test]
    fn to_team_standing_leaves_record_and_position_none_when_stats_are_missing() {
        let entry: Entry = serde_json::from_str(
            r#"{ "team": { "id": "1", "displayName": "Some Team" }, "stats": [] }"#,
        )
        .unwrap();

        let standing = to_team_standing("Some Conference", entry);
        assert!(standing.record.is_none());
        assert!(standing.standing_position.is_none());
    }
}
