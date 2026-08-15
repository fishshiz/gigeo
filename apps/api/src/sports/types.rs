//! The league allow-list and event-level gating for sports (standings/
//! record) enrichment. Scoped to the five pro majors plus NCAA football
//! and basketball -- even though the matching mechanism (`worker`) is
//! dynamic and could technically resolve a team from any league ESPN
//! covers (minor leagues, foreign) -- see `matchups_from`'s doc comment
//! for why the allow-list stays deliberately narrow. Minor leagues
//! (MiLB/AHL/G League) were evaluated and dropped: ESPN's standings API
//! returns empty for all three (confirmed live), so they'd need a
//! different, realistically paid, data source.

use crate::events::types::EventResponse;

/// One of the eight leagues this feature covers: the five pro majors plus
/// NCAA football and basketball (split men's/women's -- see
/// `major_league`'s doc comment on why Ticketmaster's own data doesn't
/// let that split happen anywhere else).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum League {
    Nba,
    Nfl,
    Nhl,
    Mlb,
    Wnba,
    NcaaFootball,
    NcaaMensBasketball,
    NcaaWomensBasketball,
}

impl League {
    /// The `league` query param value the on-demand endpoint accepts and
    /// the DB's `sports_league` enum stores -- lowercase, matching
    /// `migrations/007_sports_enrichment.sql`/`008_ncaa_sports.sql`/
    /// `009_wnba.sql`.
    pub(crate) fn as_db_str(self) -> &'static str {
        match self {
            League::Nba => "nba",
            League::Nfl => "nfl",
            League::Nhl => "nhl",
            League::Mlb => "mlb",
            League::Wnba => "wnba",
            League::NcaaFootball => "ncaa_football",
            League::NcaaMensBasketball => "ncaa_mens_basketball",
            League::NcaaWomensBasketball => "ncaa_womens_basketball",
        }
    }

    pub(crate) fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "nba" => Some(League::Nba),
            "nfl" => Some(League::Nfl),
            "nhl" => Some(League::Nhl),
            "mlb" => Some(League::Mlb),
            "wnba" => Some(League::Wnba),
            "ncaa_football" => Some(League::NcaaFootball),
            "ncaa_mens_basketball" => Some(League::NcaaMensBasketball),
            "ncaa_womens_basketball" => Some(League::NcaaWomensBasketball),
            _ => None,
        }
    }

    /// ESPN's `{sport}/{league}` URL path segment -- one unified host
    /// covers all eight leagues (unlike API-SPORTS' separate products per
    /// sport), confirmed live during this feature's design. NCAA
    /// football's path only covers FBS (~130 teams, 11 conferences) --
    /// FCS/Division II/III opponents that show up in real Ticketmaster
    /// listings (confirmed live) simply won't match, same as any other
    /// best-effort miss. See `client` module docs.
    pub(crate) fn espn_path(self) -> &'static str {
        match self {
            League::Nba => "basketball/nba",
            League::Nfl => "football/nfl",
            League::Nhl => "hockey/nhl",
            League::Mlb => "baseball/mlb",
            League::Wnba => "basketball/wnba",
            League::NcaaFootball => "football/college-football",
            League::NcaaMensBasketball => "basketball/mens-college-basketball",
            League::NcaaWomensBasketball => "basketball/womens-college-basketball",
        }
    }
}

/// One team performer worth attempting ESPN matching for.
#[derive(Debug, Clone)]
pub(crate) struct SportsTeamCandidate {
    pub name: String,
    pub ticketmaster_attraction_id: Option<String>,
    pub league: League,
}

/// Every named performer on a real matchup event for an allow-listed
/// league -- "real matchup" meaning Sports-segment, a recognized league
/// (see `major_league`), and at least two named performers.
///
/// The two-performer requirement isn't just a safety net -- it's the
/// actual mechanism for excluding non-game listings. Confirmed live for
/// the pro majors (200 real `segmentName=Sports&countryCode=US` events,
/// sampled during this feature's design): every 2-performer Sports event
/// was a literal "Team A vs Team B" matchup; every 1-performer one was a
/// non-game product -- season tickets, hospitality packages, stadium
/// tours, training camp. Both share the same `subType` ("Team"), so
/// performer count is the only signal available that tells them apart.
/// Re-confirmed live for college football specifically when that was
/// added: of a 200-event real sample, 169 (84.5%) had exactly two
/// performers -- the same signal holds.
///
/// Emits a candidate for every named performer on a qualifying event
/// (not just the first two) -- rare events with more than two listed
/// performers (a showcase/invitational, seen live in college basketball
/// listings) cost nothing extra to also attempt matching for.
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

/// The event's league, if its *explicitly marked* primary classification
/// is Sports under a recognized league. Unlike `events::
/// is_music_classified`'s permissive default (attempt the match when no
/// classification is marked primary, since missing metadata shouldn't
/// cost a real music event its enrichment), this requires an actual
/// primary classification -- a false-positive sports match risks showing
/// fabricated-looking standings on an unrelated event, a worse failure
/// mode than the conservative "skip enrichment" default costs here.
///
/// Two different detection strategies, because Ticketmaster tags these
/// two cases completely differently (confirmed live for both):
/// - **Pro majors**: `subGenre` alone identifies the league ("NBA",
///   "NFL", "NHL", "MLB", "WNBA" -- confirmed live that WNBA gets its own
///   distinct subGenre, not folded into "Basketball"/"College" the way
///   men's and women's college ball are).
/// - **College**: `subGenre` is a flat `"College"` regardless of sport --
///   real Ticketmaster college football *and* basketball events both
///   carry `subGenre=College`, so `genre` ("Football"/"Basketball") has
///   to disambiguate the sport instead. And within college basketball,
///   Ticketmaster carries no structured field distinguishing men's from
///   women's at all -- both are `genre=Basketball, subGenre=College`
///   with nothing else different -- so `is_womens_event` falls back to
///   the event's own *name*, the one place the distinction actually
///   shows up.
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
    let sub_genre = primary.sub_genre.as_ref()?.name.as_str();

    match sub_genre {
        "NBA" => return Some(League::Nba),
        "NFL" => return Some(League::Nfl),
        "NHL" => return Some(League::Nhl),
        "MLB" => return Some(League::Mlb),
        "WNBA" => return Some(League::Wnba),
        _ => {}
    }

    if !sub_genre.eq_ignore_ascii_case("college") {
        return None;
    }
    match primary.genre.as_ref()?.name.as_str() {
        "Football" => Some(League::NcaaFootball),
        "Basketball" => Some(if is_womens_event(event) {
            League::NcaaWomensBasketball
        } else {
            League::NcaaMensBasketball
        }),
        _ => None,
    }
}

/// Whether `event` is a women's college basketball game, per its own
/// name -- e.g. "...Women's Basketball", "...Womens Basketball" (both
/// spellings seen live). Defaults to men's when absent, matching the
/// unmarked-default convention Ticketmaster's own listings already use
/// (a men's game's name never says "Men's").
fn is_womens_event(event: &EventResponse) -> bool {
    event.name.to_lowercase().contains("women")
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

    fn event_named(
        name: &str,
        classifications: Option<Vec<Classification>>,
        performers: Option<Vec<Performer>>,
    ) -> EventResponse {
        EventResponse {
            name: name.to_string(),
            ..event(classifications, performers)
        }
    }

    /// A college classification -- `subGenre=College` regardless of
    /// sport, `genre` carrying the actual sport, matching what's real
    /// live (see `major_league`'s doc comment).
    fn college_classification(genre: &str, primary: bool) -> Classification {
        Classification {
            primary: Some(primary),
            segment: Some(Segment {
                id: "1".into(),
                name: "Sports".into(),
            }),
            genre: Some(Segment {
                id: "3".into(),
                name: genre.into(),
            }),
            sub_genre: Some(Segment {
                id: "2".into(),
                name: "College".into(),
            }),
            sub_type: None,
            family: None,
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
    fn matchups_from_matches_wnba_distinctly_from_college_basketball() {
        let events = vec![event(
            Some(vec![classification("Sports", Some("WNBA"), true)]),
            Some(vec![
                performer(Some("Atlanta Dream"), Some("a1")),
                performer(Some("Indiana Fever"), Some("a2")),
            ]),
        )];

        let candidates = matchups_from(&events);
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|c| c.league == League::Wnba));
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

    #[test]
    fn matchups_from_matches_ncaa_football() {
        let events = vec![event(
            Some(vec![college_classification("Football", true)]),
            Some(vec![
                performer(Some("USC Trojans Football"), Some("a1")),
                performer(Some("San Jose State Spartans Football"), Some("a2")),
            ]),
        )];

        let candidates = matchups_from(&events);
        assert_eq!(candidates.len(), 2);
        assert!(candidates.iter().all(|c| c.league == League::NcaaFootball));
    }

    #[test]
    fn matchups_from_defaults_college_basketball_to_mens() {
        let events = vec![event_named(
            "Utah State Aggies Mens Basketball vs. Denver Pioneers Mens Basketball",
            Some(vec![college_classification("Basketball", true)]),
            Some(vec![
                performer(Some("Utah State Aggies Mens Basketball"), Some("a1")),
                performer(Some("Denver Pioneers Mens Basketball"), Some("a2")),
            ]),
        )];

        let candidates = matchups_from(&events);
        assert!(
            candidates
                .iter()
                .all(|c| c.league == League::NcaaMensBasketball)
        );
    }

    #[test]
    fn matchups_from_detects_womens_college_basketball_from_event_name() {
        let events = vec![event_named(
            "UConn Huskies Women's Basketball vs. Duke Blue Devils Women's Basketball",
            Some(vec![college_classification("Basketball", true)]),
            Some(vec![
                performer(Some("UConn Huskies Women's Basketball"), Some("a1")),
                performer(Some("Duke Blue Devils Women's Basketball"), Some("a2")),
            ]),
        )];

        let candidates = matchups_from(&events);
        assert!(
            candidates
                .iter()
                .all(|c| c.league == League::NcaaWomensBasketball)
        );
    }

    #[test]
    fn matchups_from_detects_womens_college_basketball_alternate_spelling() {
        let events = vec![event_named(
            "George Washington Womens Basketball vs. Towson Tigers Womens Basketball",
            Some(vec![college_classification("Basketball", true)]),
            Some(vec![
                performer(Some("George Washington Womens Basketball"), Some("a1")),
                performer(Some("Towson Tigers Womens Basketball"), Some("a2")),
            ]),
        )];

        let candidates = matchups_from(&events);
        assert!(
            candidates
                .iter()
                .all(|c| c.league == League::NcaaWomensBasketball)
        );
    }

    #[test]
    fn matchups_from_skips_uncovered_college_sports() {
        let events = vec![event(
            Some(vec![college_classification("Volleyball", true)]),
            Some(vec![
                performer(Some("Team A"), Some("a1")),
                performer(Some("Team B"), Some("a2")),
            ]),
        )];

        assert!(matchups_from(&events).is_empty());
    }
}
