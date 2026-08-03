use crate::error::AppError;
use crate::state::AppState;
use crate::ticketmaster_stream::{PageLimit, fetch_events_near};
use chrono::Utc;
use geohash::{Coord, encode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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

/// Finds nearby Ticketmaster events and returns the unique set of artist
/// names playing within `radius_miles` over the next `window_days`.
///
/// Shared by the create-playlist handler and the periodic playlist
/// updater, so both build a playlist's artist list the same way. Fetches
/// only the first page (`PageLimit::First`) — this runs on the updater's
/// 5-minute timer across up to 25 playlists per tick, so it trades
/// completeness in dense areas for a bounded, predictable number of
/// Ticketmaster requests. `ticketmaster_stream::get_concerts_tm_stream`
/// (the map view) is the one that needs — and pays for — full coverage.
pub async fn find_artist_names_near(
    state: &AppState,
    latitude: f64,
    longitude: f64,
    radius_miles: u8,
    window_days: i64,
) -> Result<Vec<String>, AppError> {
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

    Ok(events
        .iter()
        .flat_map(|e| e.attractions.as_ref().into_iter().flatten())
        .filter_map(|a| a.name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect())
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
