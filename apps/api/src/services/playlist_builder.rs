use crate::error::AppError;
use crate::spotify::client::normalize_artist_name;
use crate::state::AppState;
use crate::ticketmaster_stream::{PageLimit, fetch_events_near};
use chrono::Utc;
use geohash::{Coord, encode};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet};

#[derive(sqlx::Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[sqlx(type_name = "playlist_visibility", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PlaylistVisibility {
    Public,
    Private,
}

#[derive(sqlx::Type, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[sqlx(type_name = "playlist_update_mode", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PlaylistUpdateMode {
    Additive,
    Destructive,
}

/// Validates a user-supplied update cadence. Unlike playlist creation
/// (which silently clamps to the nearest allowed value), editing an
/// existing playlist should reject an invalid cadence outright rather
/// than guess at intent.
pub fn validate_cadence_days(days: i16) -> Result<i16, AppError> {
    match days {
        7 | 30 | 60 => Ok(days),
        other => Err(AppError::InvalidRequest(format!(
            "update_cadence_days must be 7, 30, or 60 (got {other})"
        ))),
    }
}

/// Resolves the requested `destructive` flag on a create-playlist request
/// into the `PlaylistUpdateMode` stored for periodic updates.
///
/// `Some(true)` or `None` (omitted) both mean updates replace the
/// playlist's tracks; `Some(false)` means updates only add tracks.
pub fn resolve_update_mode(destructive: Option<bool>) -> PlaylistUpdateMode {
    if destructive.unwrap_or(true) {
        PlaylistUpdateMode::Destructive
    } else {
        PlaylistUpdateMode::Additive
    }
}

/// A nearby artist plus the human-readable date of the event that
/// surfaced them (`EventResponse::dates_pretty`), for callers that need
/// to say *when* an artist is playing rather than just who's nearby.
/// `date` is `None` when the source event had no `dates_pretty` (Ticketmaster
/// couldn't format one) — that's an existing gap in the upstream data, not
/// something this type papers over.
pub struct ArtistEvent {
    pub name: String,
    pub date: Option<String>,
}

/// Finds nearby Ticketmaster events and returns the unique set of artists
/// playing within `radius_miles` over the next `window_days`, each paired
/// with the date of the (first-seen) event that surfaced them.
///
/// Shared by the create-playlist handler and the periodic playlist
/// updater, so both build a playlist's artist list the same way. Fetches
/// only the first page (`PageLimit::First`) — this runs on the updater's
/// 5-minute timer across up to 25 playlists per tick, so it trades
/// completeness in dense areas for a bounded, predictable number of
/// Ticketmaster requests. `ticketmaster_stream::get_concerts_tm_stream`
/// (the map view) is the one that needs — and pays for — full coverage.
pub async fn find_artist_events_near(
    state: &AppState,
    latitude: f64,
    longitude: f64,
    radius_miles: u8,
    window_days: i64,
) -> Result<Vec<ArtistEvent>, AppError> {
    let geo_hash = encode(
        Coord {
            x: longitude,
            y: latitude,
        },
        6,
    )
    .map_err(|e| AppError::Internal(format!("Failed to encode geohash: {e}")))?;

    let now = Utc::now();
    let start = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let end = (now + chrono::Duration::days(window_days))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let events = fetch_events_near(
        &state.client,
        &state.ticketmaster_key,
        &geo_hash,
        radius_miles,
        &start,
        &end,
        PageLimit::First,
    )
    .await?;

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for event in &events {
        for performer in event.performers.as_ref().into_iter().flatten() {
            let Some(name) = performer.name.clone() else {
                continue;
            };
            if seen.insert(name.clone()) {
                out.push(ArtistEvent {
                    name,
                    date: event.dates_pretty.clone(),
                });
            }
        }
    }

    Ok(out)
}

/// One genre a playlist filters/prioritizes on, from `playlist_genre_priority`
/// (lower `priority` = preferred first when tracks get truncated). An empty
/// slice of these means "no filter" -- every existing playlist today, since
/// nothing has written to that table until this feature.
pub struct GenrePriority {
    pub genre: String,
    pub priority: i16,
}

/// Converts a user-supplied, order-carries-priority genre list (as
/// submitted by the create/update playlist request) into `GenrePriority`s
/// -- first genre is priority 1 (most preferred), and so on. Shared by both
/// providers' handlers so `apply_genre_filter` always sees the same
/// priority derivation `set_playlist_genre_priority` persists.
pub fn genres_to_priorities(genres: &[String]) -> Vec<GenrePriority> {
    genres
        .iter()
        .enumerate()
        .map(|(i, genre)| GenrePriority {
            genre: genre.clone(),
            priority: (i + 1) as i16,
        })
        .collect()
}

/// Filters `artist_events` down to artists matching at least one of
/// `genre_priorities` (case-insensitive), then sorts survivors by the best
/// (lowest) priority among their matching genres -- ties broken by original
/// discovery order. An artist absent from `genres_by_normalized` (not yet
/// enriched, or enriched with no genres) never matches a non-empty filter.
///
/// Split out from `apply_genre_filter` so the matching/sorting logic itself
/// is testable without a live database, mirroring the
/// `attach_genres`/`merge_genres` split in `artists::lookup`.
fn filter_and_sort_by_genre(
    artist_events: Vec<ArtistEvent>,
    genre_priorities: &[GenrePriority],
    genres_by_normalized: &HashMap<String, Vec<String>>,
) -> Vec<ArtistEvent> {
    if genre_priorities.is_empty() {
        return artist_events;
    }

    let wanted: HashMap<String, i16> = genre_priorities
        .iter()
        .map(|g| (g.genre.to_lowercase(), g.priority))
        .collect();

    let mut matched: Vec<(i16, usize, ArtistEvent)> = artist_events
        .into_iter()
        .enumerate()
        .filter_map(|(i, artist)| {
            let genres = genres_by_normalized.get(&normalize_artist_name(&artist.name))?;
            let best_priority = genres
                .iter()
                .filter_map(|g| wanted.get(&g.to_lowercase()))
                .min()
                .copied()?;
            Some((best_priority, i, artist))
        })
        .collect();

    matched.sort_by_key(|(priority, i, _)| (*priority, *i));
    matched.into_iter().map(|(_, _, artist)| artist).collect()
}

/// DB-backed wrapper around [`filter_and_sort_by_genre`]: looks up genres
/// for every artist in `artist_events`, then filters/sorts by
/// `genre_priorities`. Best-effort like `artists::attach_genres` -- a DB
/// error surfaces as "no genre data available", which `filter_and_sort_by_genre`
/// then treats as no matches for a non-empty filter. This means a DB hiccup
/// on a genre-filtered playlist produces an empty result rather than
/// silently ignoring the user's filter and returning everything; the
/// create/update call sites already treat "no artists found" as a normal,
/// user-visible outcome (unlike `attach_genres`, which only decorates
/// already-successful responses).
pub async fn apply_genre_filter(
    pool: &PgPool,
    artist_events: Vec<ArtistEvent>,
    genre_priorities: &[GenrePriority],
) -> Vec<ArtistEvent> {
    if genre_priorities.is_empty() {
        return artist_events;
    }

    let names: Vec<String> = artist_events.iter().map(|a| a.name.clone()).collect();
    let genres_by_normalized = crate::artists::genres_for_names(pool, &names).await;

    filter_and_sort_by_genre(artist_events, genre_priorities, &genres_by_normalized)
}

pub struct TrackResult {
    pub name: String,
    pub artist: String,
    pub uri: String,
}

/// Searches Spotify (client-credentials token) for up to `per_artist` top
/// tracks per artist name.
///
/// Shared by the create-playlist handler and the periodic playlist
/// updater.
pub async fn search_tracks_for_artists(
    state: &AppState,
    cc_access_token: &str,
    artist_names: &[String],
    per_artist: u8,
) -> Result<Vec<TrackResult>, AppError> {
    let mut results = Vec::new();

    for artist_name in artist_names {
        if let Some(tracks) = state
            .spotify
            .search_tracks_by_artist(cc_access_token, artist_name, per_artist)
            .await?
        {
            for t in tracks {
                results.push(TrackResult {
                    name: t.name,
                    artist: t
                        .artists
                        .first()
                        .map(|a| a.name.clone())
                        .unwrap_or_default(),
                    uri: t.uri,
                });
            }
        }
    }

    Ok(results)
}

/// Searches Apple Music's catalog (developer token only, no user token
/// needed for catalog search) for up to `per_artist` top tracks per artist
/// name. Mirrors `search_tracks_for_artists`; `TrackResult::uri` holds the
/// Apple catalog song ID rather than a Spotify URI.
///
/// Shared by the create-playlist handler and the periodic playlist
/// updater's Apple Music path.
pub async fn search_tracks_for_artists_apple(
    state: &AppState,
    developer_token: &str,
    artist_names: &[String],
    per_artist: u8,
) -> Result<Vec<TrackResult>, AppError> {
    let am = state
        .apple_music_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music client not configured".into()))?;

    let mut results = Vec::new();

    for artist_name in artist_names {
        let songs = am
            .search_songs(developer_token, artist_name, per_artist)
            .await?;

        for song in songs {
            let attrs = song.attributes.as_ref();
            results.push(TrackResult {
                name: attrs.map(|a| a.name.clone()).unwrap_or_default(),
                artist: attrs.map(|a| a.artist_name.clone()).unwrap_or_default(),
                uri: song.id,
            });
        }
    }

    Ok(results)
}

/// Spotify's hard cap on a playlist description's length. Apple Music
/// library playlist descriptions aren't documented with a matching limit,
/// but this is applied there too for one consistent format.
const DESCRIPTION_CHAR_LIMIT: usize = 300;

/// Builds a bulleted "Artist — Date" playlist description from a set of
/// nearby artist events (in discovery order), stopping before
/// `DESCRIPTION_CHAR_LIMIT` and appending a "+N more" line for whatever
/// didn't fit. Always includes at least one bullet when `artists` is
/// non-empty, even if that single bullet alone would exceed the limit — a
/// fuller-than-ideal description beats an empty one.
///
/// Used both at playlist creation (so a brand-new playlist explains which
/// nearby artists populated it) and by the periodic destructive-update job
/// (`playlist_updater::do_update_spotify`), which regenerates it every
/// replace since the prior description described tracks that no longer
/// exist.
pub fn build_bulleted_description(artists: &[ArtistEvent]) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut used = 0usize;

    for (i, artist) in artists.iter().enumerate() {
        let bullet = match &artist.date {
            Some(date) => format!("• {} — {date}", artist.name),
            None => format!("• {}", artist.name),
        };

        let remaining_after = artists.len() - (i + 1);
        let more_suffix_len = if remaining_after > 0 {
            1 + format!("+{remaining_after} more").len()
        } else {
            0
        };
        let bullet_len = bullet.len() + if lines.is_empty() { 0 } else { 1 };

        if !lines.is_empty() && used + bullet_len + more_suffix_len > DESCRIPTION_CHAR_LIMIT {
            break;
        }

        used += bullet_len;
        lines.push(bullet);
    }

    let remaining = artists.len() - lines.len();
    if remaining > 0 {
        lines.push(format!("+{remaining} more"));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artist(name: &str, date: Option<&str>) -> ArtistEvent {
        ArtistEvent {
            name: name.to_string(),
            date: date.map(str::to_string),
        }
    }

    #[test]
    fn resolve_update_mode_true_is_destructive() {
        assert_eq!(
            resolve_update_mode(Some(true)),
            PlaylistUpdateMode::Destructive
        );
    }

    #[test]
    fn resolve_update_mode_false_is_additive() {
        assert_eq!(
            resolve_update_mode(Some(false)),
            PlaylistUpdateMode::Additive
        );
    }

    #[test]
    fn resolve_update_mode_none_defaults_to_destructive() {
        assert_eq!(resolve_update_mode(None), PlaylistUpdateMode::Destructive);
    }

    #[test]
    fn validate_cadence_days_accepts_allowed_values() {
        assert_eq!(validate_cadence_days(7).unwrap(), 7);
        assert_eq!(validate_cadence_days(30).unwrap(), 30);
        assert_eq!(validate_cadence_days(60).unwrap(), 60);
    }

    #[test]
    fn validate_cadence_days_rejects_other_values() {
        assert!(validate_cadence_days(14).is_err());
        assert!(validate_cadence_days(0).is_err());
        assert!(validate_cadence_days(-7).is_err());
    }

    #[test]
    fn build_bulleted_description_empty_artists_is_empty_string() {
        assert_eq!(build_bulleted_description(&[]), "");
    }

    #[test]
    fn build_bulleted_description_bullets_artist_and_date() {
        let artists = vec![artist("Phoebe Bridgers", Some("Aug 20"))];
        assert_eq!(
            build_bulleted_description(&artists),
            "• Phoebe Bridgers — Aug 20"
        );
    }

    #[test]
    fn build_bulleted_description_falls_back_without_date() {
        let artists = vec![artist("Phoebe Bridgers", None)];
        assert_eq!(build_bulleted_description(&artists), "• Phoebe Bridgers");
    }

    #[test]
    fn build_bulleted_description_fits_everything_when_under_limit() {
        let artists = vec![
            artist("Artist One", Some("Aug 20")),
            artist("Artist Two", Some("Aug 21")),
        ];
        let desc = build_bulleted_description(&artists);
        assert_eq!(desc, "• Artist One — Aug 20\n• Artist Two — Aug 21");
        assert!(desc.len() <= DESCRIPTION_CHAR_LIMIT);
    }

    #[test]
    fn build_bulleted_description_never_exceeds_char_limit() {
        // Enough long-named artists that the full bulleted list would
        // blow well past 300 chars.
        let artists: Vec<ArtistEvent> = (0..40)
            .map(|i| {
                artist(
                    &format!("A Fairly Long Artist Name Number {i}"),
                    Some("Aug 20"),
                )
            })
            .collect();
        let desc = build_bulleted_description(&artists);
        assert!(desc.len() <= DESCRIPTION_CHAR_LIMIT, "len={}", desc.len());
        assert!(desc.contains("more"));
    }

    #[test]
    fn build_bulleted_description_overflow_appends_more_line_not_partial_bullet() {
        // A third artist that doesn't fit alongside the first two should
        // surface as a whole "+N more" line, never a bullet truncated
        // mid-word.
        let long_name = "X".repeat(130);
        let artists = vec![
            artist(&long_name, Some("Aug 20")),
            artist(&long_name, Some("Aug 21")),
            artist("One More Artist", Some("Aug 22")),
        ];
        let desc = build_bulleted_description(&artists);
        assert!(desc.len() <= DESCRIPTION_CHAR_LIMIT, "len={}", desc.len());
        let last_line = desc.lines().last().unwrap();
        assert!(
            last_line.starts_with('+') && last_line.ends_with(" more"),
            "expected a '+N more' summary line, got {last_line:?}"
        );
        assert!(!desc.contains("One More Artist"));
    }

    fn genre_priority(genre: &str, priority: i16) -> GenrePriority {
        GenrePriority {
            genre: genre.to_string(),
            priority,
        }
    }

    #[test]
    fn filter_and_sort_by_genre_empty_filter_is_passthrough() {
        let artists = vec![artist("Role Model", None), artist("Someone Else", None)];
        let result = filter_and_sort_by_genre(artists, &[], &HashMap::new());
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Role Model");
        assert_eq!(result[1].name, "Someone Else");
    }

    #[test]
    fn filter_and_sort_by_genre_drops_non_matching_artists() {
        let artists = vec![artist("Role Model", None), artist("Someone Else", None)];
        let mut genres = HashMap::new();
        genres.insert("rolemodel".to_string(), vec!["Pop".to_string()]);
        genres.insert("someoneelse".to_string(), vec!["Metal".to_string()]);

        let result = filter_and_sort_by_genre(artists, &[genre_priority("Pop", 1)], &genres);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Role Model");
    }

    #[test]
    fn filter_and_sort_by_genre_drops_artists_with_no_genre_data() {
        let artists = vec![artist("Unenriched Artist", None)];
        let result =
            filter_and_sort_by_genre(artists, &[genre_priority("Pop", 1)], &HashMap::new());
        assert!(result.is_empty());
    }

    #[test]
    fn filter_and_sort_by_genre_is_case_insensitive() {
        let artists = vec![artist("Role Model", None)];
        let mut genres = HashMap::new();
        genres.insert("rolemodel".to_string(), vec!["pop".to_string()]);

        let result = filter_and_sort_by_genre(artists, &[genre_priority("POP", 1)], &genres);

        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_and_sort_by_genre_orders_by_best_matching_priority() {
        let artists = vec![
            artist("Rock Artist", None),
            artist("Pop Artist", None),
            artist("Both Genres Artist", None),
        ];
        let mut genres = HashMap::new();
        genres.insert("rockartist".to_string(), vec!["Rock".to_string()]);
        genres.insert("popartist".to_string(), vec!["Pop".to_string()]);
        genres.insert(
            "bothgenresartist".to_string(),
            vec!["Rock".to_string(), "Pop".to_string()],
        );

        // Pop is priority 1 (preferred), Rock is priority 2.
        let priorities = [genre_priority("Pop", 1), genre_priority("Rock", 2)];
        let result = filter_and_sort_by_genre(artists, &priorities, &genres);

        assert_eq!(result.len(), 3);
        // Pop Artist and Both Genres Artist both have a priority-1 match
        // (Pop) -- discovery order breaks the tie ahead of Rock Artist's
        // priority-2-only match.
        assert_eq!(result[0].name, "Pop Artist");
        assert_eq!(result[1].name, "Both Genres Artist");
        assert_eq!(result[2].name, "Rock Artist");
    }

    #[test]
    fn build_bulleted_description_single_oversized_artist_still_included() {
        // A single artist whose bullet alone exceeds the cap must still
        // produce a non-empty description rather than an empty one.
        let long_name = "Y".repeat(400);
        let artists = vec![artist(&long_name, Some("Aug 20"))];
        let desc = build_bulleted_description(&artists);
        assert!(desc.contains(&long_name));
    }
}
