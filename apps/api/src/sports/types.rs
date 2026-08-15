//! The major-league allow-list and event-level gating for sports
//! (standings/record) enrichment. Scoped to the four major US leagues for
//! v1 even though the matching mechanism (`worker`) is dynamic and could
//! technically resolve a team from any league ESPN covers (college,
//! semi-pro, foreign) -- see `matchups_from`'s doc comment for why that's
//! deliberate.

use crate::events::types::EventResponse;

/// One of the four major US leagues this feature is scoped to for v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum League {
    Nba,
    Nfl,
    Nhl,
    Mlb,
}

impl League {
    /// Matches a Ticketmaster `subGenre.name`. Values confirmed live
    /// against real Sports-segment events during this feature's design --
    /// see the design conversation this shipped from for the sample (200
    /// real events, `segmentName=Sports&countryCode=US`).
    pub(crate) fn from_subgenre(name: &str) -> Option<Self> {
        match name {
            "NBA" => Some(League::Nba),
            "NFL" => Some(League::Nfl),
            "NHL" => Some(League::Nhl),
            "MLB" => Some(League::Mlb),
            _ => None,
        }
    }

    /// The `league` query param value the on-demand endpoint accepts and
    /// the DB's `sports_league` enum stores -- lowercase, matching
    /// `migrations/007_sports_enrichment.sql`.
    pub(crate) fn as_db_str(self) -> &'static str {
        match self {
            League::Nba => "nba",
            League::Nfl => "nfl",
            League::Nhl => "nhl",
            League::Mlb => "mlb",
        }
    }

    pub(crate) fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "nba" => Some(League::Nba),
            "nfl" => Some(League::Nfl),
            "nhl" => Some(League::Nhl),
            "mlb" => Some(League::Mlb),
            _ => None,
        }
    }

    /// ESPN's `{sport}/{league}` URL path segment -- one unified host
    /// covers all four leagues (unlike API-SPORTS' four separate
    /// products), confirmed live during this feature's design. See
    /// `client` module docs.
    pub(crate) fn espn_path(self) -> &'static str {
        match self {
            League::Nba => "basketball/nba",
            League::Nfl => "football/nfl",
            League::Nhl => "hockey/nhl",
            League::Mlb => "baseball/mlb",
        }
    }
}

/// One team performer worth attempting API-SPORTS matching for.
#[derive(Debug, Clone)]
pub(crate) struct SportsTeamCandidate {
    pub name: String,
    pub ticketmaster_attraction_id: Option<String>,
    pub league: League,
}

/// Every named performer on a real major-league matchup event -- "real
/// matchup" meaning Sports-segment, a `subGenre` in the major-league
/// allow-list, and at least two named performers.
///
/// The two-performer requirement isn't just a safety net -- it's the
/// actual mechanism for excluding non-game listings. Confirmed live
/// (200 real `segmentName=Sports&countryCode=US` events, sampled during
/// this feature's design): every 2-performer Sports event was a literal
/// "Team A vs Team B" matchup; every 1-performer one was a non-game
/// product -- season tickets, hospitality packages, stadium tours,
/// training camp. Both share the same `subType` ("Team"), so performer
/// count is the only signal available that tells them apart.
///
/// Emits a candidate for every named performer on a qualifying event
/// (not just the first two) -- rare events with more than two listed
/// performers cost nothing extra to also attempt matching for.
pub(crate) fn matchups_from(events: &[EventResponse]) -> Vec<SportsTeamCandidate> {
    events
        .iter()
        .filter_map(|event| {
            let league = major_league(event)?;
            let performers = event.performers.as_ref()?;
            let named: Vec<_> = performers.iter().filter(|p| p.name.is_some()).collect();
            if named.len() < 2 {
                return None;
            }
            Some(named.into_iter().map(move |p| SportsTeamCandidate {
                #[allow(clippy::unwrap_used)]
                name: p.name.clone().unwrap(),
                ticketmaster_attraction_id: p.id.clone(),
                league,
            }))
        })
        .flatten()
        .collect()
}

/// The event's major league, if its *explicitly marked* primary
/// classification is Sports under an allow-listed subGenre. Unlike
/// `events::is_music_classified`'s permissive default (attempt the match
/// when no classification is marked primary, since missing metadata
/// shouldn't cost a real music event its enrichment), this requires an
/// actual primary classification -- a false-positive sports match risks
/// showing fabricated-looking standings on an unrelated event, a worse
/// failure mode than the conservative "skip enrichment" default costs
/// here.
fn major_league(event: &EventResponse) -> Option<League> {
    let classifications = event.classifications.as_ref()?;
    let primary = classifications.iter().find(|c| c.primary == Some(true))?;
    let is_sports = primary
        .segment
        .as_ref()
        .is_some_and(|s| s.name.eq_ignore_ascii_case("sports"));
    if !is_sports {
        return None;
    }
    League::from_subgenre(&primary.sub_genre.as_ref()?.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{Classification, Performer, Segment, Source, VenueResponse};

    fn classification(segment: &str, sub_genre: Option<&str>, primary: bool) -> Classification {
        Classification {
            primary: Some(primary),
            segment: Some(Segment {
                id: "1".into(),
                name: segment.into(),
            }),
            genre: None,
            sub_genre: sub_genre.map(|s| Segment {
                id: "2".into(),
                name: s.into(),
            }),
            sub_type: None,
            family: None,
        }
    }

    fn performer(name: Option<&str>, id: Option<&str>) -> Performer {
        Performer {
            name: name.map(str::to_string),
            id: id.map(str::to_string),
            classifications: None,
            external_links: None,
            images: None,
            genres: vec![],
        }
    }

    fn event(classifications: Option<Vec<Classification>>, performers: Option<Vec<Performer>>) -> EventResponse {
        EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: None::<VenueResponse>,
            images: vec![],
            dates: None,
            dates_pretty: None,
            local_calendar_day: None,
            classifications,
            performers,
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        }
    }

    #[test]
    fn matchups_from_matches_a_two_performer_nba_game() {
        let events = vec![event(
            Some(vec![classification("Sports", Some("NBA"), true)]),
            Some(vec![
                performer(Some("Lakers"), Some("a1")),
                performer(Some("Celtics"), Some("a2")),
            ]),
        )];

        let candidates = matchups_from(&events);
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|c| c.league == League::Nba));
    }

    #[test]
    fn matchups_from_skips_single_performer_events() {
        let events = vec![event(
            Some(vec![classification("Sports", Some("MLB"), true)]),
            Some(vec![performer(Some("Mariners"), Some("a1"))]),
        )];

        assert!(matchups_from(&events).is_empty());
    }

    #[test]
    fn matchups_from_skips_non_major_league_sports() {
        let events = vec![event(
            Some(vec![classification("Sports", Some("Tennis"), true)]),
            Some(vec![
                performer(Some("Player A"), Some("a1")),
                performer(Some("Player B"), Some("a2")),
            ]),
        )];

        assert!(matchups_from(&events).is_empty());
    }

    #[test]
    fn matchups_from_skips_non_sports_events() {
        let events = vec![event(
            Some(vec![classification("Music", None, true)]),
            Some(vec![
                performer(Some("Artist A"), Some("a1")),
                performer(Some("Artist B"), Some("a2")),
            ]),
        )];

        assert!(matchups_from(&events).is_empty());
    }

    #[test]
    fn matchups_from_requires_an_explicitly_marked_primary_classification() {
        let events = vec![event(
            Some(vec![classification("Sports", Some("NFL"), false)]),
            Some(vec![
                performer(Some("Team A"), Some("a1")),
                performer(Some("Team B"), Some("a2")),
            ]),
        )];

        assert!(matchups_from(&events).is_empty());
    }

    #[test]
    fn matchups_from_skips_unnamed_performers_when_counting() {
        let events = vec![event(
            Some(vec![classification("Sports", Some("NHL"), true)]),
            Some(vec![performer(Some("Bruins"), Some("a1")), performer(None, Some("a2"))]),
        )];

        assert!(matchups_from(&events).is_empty());
    }
}
