//! Read path for attaching persisted canonical-artist data to outgoing
//! event responses — the counterpart to `worker`'s write path. This is a
//! plain indexed `SELECT`, not a call to Apple Music/Spotify, so unlike
//! `worker::spawn_enrichment` it runs inline in the live request rather
//! than detached: one batched query per page of events (keyed by
//! normalized name) rather than one round-trip per performer.

use std::collections::HashMap;

use chrono::{Duration, NaiveDate, Utc};
use sqlx::PgPool;
use sqlx::types::Json;

use super::SimilarArtist;
use crate::events::types::{
    ArtistArtwork, ArtistEnrichment, EventResponse, RecentRelease, SimilarArtistResponse,
};
use crate::spotify::client::normalize_artist_name;

/// How recent `latest_release_date` has to be, as of "now" at response
/// time, to surface as `ArtistEnrichment::recent_release` — see #102's
/// locked decision. A read-time window rather than a write-time filter:
/// `artists::worker::backfill_release_info` stores whatever the actual
/// latest release is regardless of age, so this can change later without
/// needing every artist re-fetched.
const RECENT_RELEASE_WINDOW: Duration = Duration::days(30);

struct EnrichmentRow {
    normalized_name: String,
    display_name: String,
    apple_music_id: Option<String>,
    spotify_id: Option<String>,
    artwork_url: Option<String>,
    artwork_bg_color: Option<String>,
    apple_music_url: Option<String>,
    spotify_url: Option<String>,
    genres: Vec<String>,
    similar_artists: Json<Vec<SimilarArtist>>,
    popularity: Option<i16>,
    latest_release_name: Option<String>,
    latest_release_date: Option<NaiveDate>,
    latest_release_url: Option<String>,
    latest_release_artwork_url: Option<String>,
}

/// Attaches persisted genres (see ADR-0001) to every performer in `events`
/// that has a matched row -- the list/map stream's own trimmed need (the
/// "for you" genre filter, `apps/web/src/providers/eventsProvider.tsx`'s
/// `matchedPerformerGenres`), not the full `ArtistEnrichment` (artwork,
/// similar artists, display name, provider urls), which now comes from the
/// on-demand `/artists/enrichment` endpoint instead (`fetch_one`, below),
/// fetched only when a user actually opens an event's details. Best-effort
/// throughout: a performer with no row (not yet resolved, or resolved to no
/// match), or a DB error, is simply left with empty `genres` — this must
/// never fail or slow down event streaming, matching every other step in
/// this pipeline.
pub(crate) async fn attach_genres(events: &mut [EventResponse], pool: &PgPool) {
    let mut names_by_normalized: HashMap<String, ()> = HashMap::new();
    for event in events.iter() {
        for performer in event.performers.iter().flatten() {
            if let Some(name) = &performer.name {
                let normalized = normalize_artist_name(name);
                if !normalized.is_empty() {
                    names_by_normalized.insert(normalized, ());
                }
            }
        }
    }

    if names_by_normalized.is_empty() {
        return;
    }

    let normalized_names: Vec<String> = names_by_normalized.into_keys().collect();

    let rows = match fetch_rows(pool, &normalized_names).await {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!(error = %err, "artist enrichment: failed to read rows for event response, skipping");
            return;
        }
    };

    let genres_by_normalized: HashMap<String, Vec<String>> = rows
        .iter()
        .map(|row| (row.normalized_name.clone(), row.genres.clone()))
        .collect();

    merge_genres(events, &genres_by_normalized);
}

/// Splits out from `attach_genres` so the merge itself (as opposed to the
/// DB round-trip) is testable without a live database.
fn merge_genres(events: &mut [EventResponse], genres_by_normalized: &HashMap<String, Vec<String>>) {
    for event in events.iter_mut() {
        for performer in event.performers.iter_mut().flatten() {
            let Some(name) = &performer.name else {
                continue;
            };
            let normalized = normalize_artist_name(name);
            performer.genres = genres_by_normalized
                .get(&normalized)
                .cloned()
                .unwrap_or_default();
        }
    }
}

/// Reads back one performer's full persisted enrichment, if matched — the
/// read half of the on-demand `/artists/enrichment` endpoint
/// (`artists::get_performer_enrichment`), called after `worker::resolve_one`
/// (and, for a still-missing similar-artists list,
/// `worker::backfill_similar_artists`) have ensured the row is as complete
/// as it can be made right now.
pub(crate) async fn fetch_one(pool: &PgPool, normalized_name: &str) -> Option<ArtistEnrichment> {
    let names = [normalized_name.to_string()];
    let rows = fetch_rows(pool, &names).await.ok()?;
    rows.first().map(to_response)
}

async fn fetch_rows(
    pool: &PgPool,
    normalized_names: &[String],
) -> Result<Vec<EnrichmentRow>, sqlx::Error> {
    sqlx::query_as!(
        EnrichmentRow,
        r#"
        select
            normalized_name,
            display_name,
            apple_music_id,
            spotify_id,
            artwork_url,
            artwork_bg_color,
            apple_music_url,
            spotify_url,
            genres,
            similar_artists as "similar_artists: Json<Vec<SimilarArtist>>",
            popularity,
            latest_release_name,
            latest_release_date,
            latest_release_url,
            latest_release_artwork_url
        from artists
        where normalized_name = any($1) and matched_via is not null
        "#,
        normalized_names
    )
    .fetch_all(pool)
    .await
}

fn to_response(row: &EnrichmentRow) -> ArtistEnrichment {
    let id = row
        .apple_music_id
        .clone()
        .or_else(|| row.spotify_id.clone())
        .unwrap_or_default();

    let artwork = row.artwork_url.clone().map(|url| ArtistArtwork {
        url,
        bg_color: row.artwork_bg_color.clone(),
    });

    let recent_release =
        row.latest_release_date
            .filter(is_recent)
            .and(row.latest_release_name.clone().map(|name| {
                RecentRelease {
                    name,
                    url: row.latest_release_url.clone(),
                    artwork: row
                        .latest_release_artwork_url
                        .clone()
                        .map(|url| ArtistArtwork {
                            url,
                            bg_color: None,
                        }),
                }
            }));

    ArtistEnrichment {
        name: row.display_name.clone(),
        id,
        apple_music_url: row.apple_music_url.clone(),
        spotify_url: row.spotify_url.clone(),
        artwork,
        genres: row.genres.clone(),
        similar_artists: row
            .similar_artists
            .0
            .iter()
            .map(|s| SimilarArtistResponse {
                name: s.name.clone(),
                id: s.apple_music_id.clone(),
                apple_music_url: s.apple_music_url.clone(),
                artwork: s.artwork_url.clone().map(|url| ArtistArtwork {
                    url,
                    bg_color: None,
                }),
            })
            .collect(),
        popularity: row.popularity.map(|p| p as u8),
        recent_release,
    }
}

/// Whether `date` falls within `RECENT_RELEASE_WINDOW` of now — split out
/// so `to_response`'s `.filter(...)` reads as "recent" at the call site.
fn is_recent(date: &NaiveDate) -> bool {
    Utc::now().date_naive() - *date < RECENT_RELEASE_WINDOW
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{Performer, Source, VenueResponse};

    fn row(normalized_name: &str) -> EnrichmentRow {
        EnrichmentRow {
            normalized_name: normalized_name.to_string(),
            display_name: "Role Model".to_string(),
            apple_music_id: Some("am-1".to_string()),
            spotify_id: None,
            artwork_url: Some("https://example.com/art.jpg".to_string()),
            artwork_bg_color: Some("abcdef".to_string()),
            apple_music_url: Some("https://music.apple.com/artist/1".to_string()),
            spotify_url: None,
            genres: vec!["Pop".to_string()],
            similar_artists: Json(vec![SimilarArtist {
                name: "Similar Artist".to_string(),
                apple_music_id: "am-2".to_string(),
                apple_music_url: Some("https://music.apple.com/artist/2".to_string()),
                artwork_url: Some("https://example.com/similar.jpg".to_string()),
            }]),
            popularity: Some(72),
            latest_release_name: None,
            latest_release_date: None,
            latest_release_url: None,
            latest_release_artwork_url: None,
        }
    }

    #[test]
    fn to_response_prefers_apple_music_id_over_spotify_id() {
        let mut r = row("role model");
        r.apple_music_id = Some("am-1".to_string());
        r.spotify_id = Some("sp-1".to_string());
        assert_eq!(to_response(&r).id, "am-1");
    }

    #[test]
    fn to_response_falls_back_to_spotify_id_when_no_apple_music_id() {
        let mut r = row("role model");
        r.apple_music_id = None;
        r.spotify_id = Some("sp-1".to_string());
        assert_eq!(to_response(&r).id, "sp-1");
    }

    #[test]
    fn to_response_leaves_artwork_none_when_no_artwork_url() {
        let mut r = row("role model");
        r.artwork_url = None;
        assert!(to_response(&r).artwork.is_none());
    }

    #[test]
    fn to_response_maps_spotify_url_when_present() {
        let mut r = row("role model");
        r.spotify_url = Some("https://open.spotify.com/artist/1".to_string());
        assert_eq!(
            to_response(&r).spotify_url.as_deref(),
            Some("https://open.spotify.com/artist/1")
        );
    }

    #[test]
    fn to_response_leaves_spotify_url_none_when_absent() {
        let r = row("role model");
        assert!(to_response(&r).spotify_url.is_none());
    }

    #[test]
    fn to_response_maps_popularity() {
        let r = row("role model");
        assert_eq!(to_response(&r).popularity, Some(72));
    }

    #[test]
    fn to_response_omits_recent_release_when_none_stored() {
        let r = row("role model");
        assert!(to_response(&r).recent_release.is_none());
    }

    #[test]
    fn to_response_surfaces_recent_release_within_window() {
        let mut r = row("role model");
        r.latest_release_name = Some("New Album".to_string());
        r.latest_release_date = Some(Utc::now().date_naive() - Duration::days(5));
        r.latest_release_url = Some("https://open.spotify.com/album/1".to_string());
        r.latest_release_artwork_url = Some("https://example.com/album.jpg".to_string());

        let release = to_response(&r).recent_release.expect("should be recent");
        assert_eq!(release.name, "New Album");
        assert_eq!(
            release.url.as_deref(),
            Some("https://open.spotify.com/album/1")
        );
        assert_eq!(
            release.artwork.as_ref().unwrap().url,
            "https://example.com/album.jpg"
        );
    }

    #[test]
    fn to_response_omits_release_older_than_window() {
        let mut r = row("role model");
        r.latest_release_name = Some("Old Album".to_string());
        r.latest_release_date = Some(Utc::now().date_naive() - Duration::days(45));

        assert!(to_response(&r).recent_release.is_none());
    }

    #[test]
    fn to_response_maps_similar_artists() {
        let response = to_response(&row("role model"));
        assert_eq!(response.similar_artists.len(), 1);
        assert_eq!(response.similar_artists[0].id, "am-2");
        assert_eq!(
            response.similar_artists[0].artwork.as_ref().unwrap().url,
            "https://example.com/similar.jpg"
        );
    }

    fn performer(name: Option<&str>) -> Performer {
        Performer {
            name: name.map(str::to_string),
            id: None,
            classifications: None,
            external_links: None,
            images: None,
            genres: vec![],
        }
    }

    fn event_with_performers(names: &[&str]) -> EventResponse {
        EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: None,
                location: None,
                city: None,
                state: None,
                state_code: None,
                address: None,
                postal_code: None,
                url: None,
                images: vec![],
            }),
            images: vec![],
            dates: None,
            dates_pretty: None,
            local_calendar_day: None,
            classifications: None,
            performers: Some(names.iter().map(|n| performer(Some(n))).collect()),
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
    fn merge_genres_matches_by_normalized_name() {
        let mut events = vec![event_with_performers(&["Role Model"])];
        let mut by_normalized = HashMap::new();
        by_normalized.insert("rolemodel".to_string(), vec!["Pop".to_string()]);

        merge_genres(&mut events, &by_normalized);

        assert_eq!(
            events[0].performers.as_ref().unwrap()[0].genres,
            vec!["Pop".to_string()]
        );
    }

    #[test]
    fn merge_genres_leaves_unmatched_performers_empty() {
        let mut events = vec![event_with_performers(&["Someone Else"])];
        let mut by_normalized = HashMap::new();
        by_normalized.insert("rolemodel".to_string(), vec!["Pop".to_string()]);

        merge_genres(&mut events, &by_normalized);

        assert!(events[0].performers.as_ref().unwrap()[0].genres.is_empty());
    }

    #[test]
    fn merge_genres_skips_performers_with_no_name_without_panicking() {
        let mut events = vec![event_with_performers(&[])];
        events[0].performers = Some(vec![performer(None)]);

        merge_genres(&mut events, &HashMap::new());

        assert!(events[0].performers.as_ref().unwrap()[0].genres.is_empty());
    }
}
