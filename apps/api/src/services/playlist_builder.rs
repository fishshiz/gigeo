use crate::error::AppError;
use crate::state::AppState;
use crate::ticketmaster_stream::{
    EventResponse, EventsQuery, TicketmasterResponse, dedupe_key, normalize_event,
};
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

pub async fn get_concerts_tm_impl(
    state: &AppState,
    params: &EventsQuery,
) -> Result<Vec<EventResponse>, AppError> {
    let hash = encode(
        Coord {
            x: params.longitude,
            y: params.latitude,
        },
        6,
    )
    .map_err(|e| AppError::Internal(format!("Failed to encode geohash: {e}")))?;

    let url = format!(
        "https://app.ticketmaster.com/discovery/v2/events.json?geoPoint={}&apikey={}&radius={}&startDateTime={}&endDateTime={}&size=200&sort=date,asc",
        hash, state.ticketmaster_key, params.radius, params.start, params.end
    );

    let resp = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(AppError::Request)?;

    let status = resp.status();
    let text = resp.text().await.map_err(AppError::Request)?;

    if !status.is_success() {
        return Err(AppError::TicketmasterApi {
            status,
            message: text,
        });
    }

    let body: TicketmasterResponse = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("Ticketmaster decode error: {e}")))?;

    let mut seen = HashSet::<String>::new();
    let events: Vec<EventResponse> = body
        .embedded
        .into_iter()
        .flat_map(|e| e.events)
        .map(normalize_event)
        .filter(|event| seen.insert(dedupe_key(event)))
        .collect();

    Ok(events)
}

/// Finds nearby Ticketmaster events and returns the unique set of artist
/// names playing within `radius_miles` over the next `window_days`.
///
/// Shared by the create-playlist handler and the periodic playlist
/// updater, so both build a playlist's artist list the same way.
pub async fn find_artist_names_near(
    state: &AppState,
    latitude: f64,
    longitude: f64,
    radius_miles: u8,
    window_days: i64,
) -> Result<Vec<String>, AppError> {
    let now = Utc::now();
    let query = EventsQuery {
        latitude,
        longitude,
        radius: radius_miles,
        start: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        end: (now + chrono::Duration::days(window_days))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string(),
    };

    let events = get_concerts_tm_impl(state, &query).await?;

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
