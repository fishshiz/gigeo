//! Apple Music API client.
//!
//! Endpoint references:
//!   Search       – https://developer.apple.com/documentation/applemusicapi/search_for_catalog_resources
//!   Get Artist   – https://developer.apple.com/documentation/applemusicapi/get-a-catalog-artist
//!   Artist Views – https://developer.apple.com/documentation/applemusicapi/artists/views-data.dictionary
//!   Create Playlist  – https://developer.apple.com/documentation/applemusicapi/create-a-new-library-playlist
//!   Add Tracks       – https://developer.apple.com/documentation/applemusicapi/add-tracks-to-a-library-playlist
//!
//! All endpoint paths and field names are taken from the official Apple documentation.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::http_utils::{request_with_backoff, url_encode};

const BASE: &str = "https://api.music.apple.com";

// ---------------------------------------------------------------------------
// Domain types (matching Apple Music API resource schemas)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artwork {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(rename = "bgColor")]
    pub bg_color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtistAttributes {
    pub name: String,
    #[serde(rename = "genreNames", default)]
    pub genre_names: Vec<String>,
    pub artwork: Option<Artwork>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArtistResource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub href: Option<String>,
    pub attributes: Option<ArtistAttributes>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SongAttributes {
    pub name: String,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    #[serde(rename = "durationInMillis")]
    pub duration_in_millis: Option<u64>,
    #[serde(rename = "trackNumber")]
    pub track_number: Option<u32>,
    pub artwork: Option<Artwork>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SongResource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: Option<SongAttributes>,
}

// -- Response wrappers ------------------------------------------------------

/// Generic wrapper for Apple Music responses with a `data` array.
#[derive(Debug, Deserialize)]
pub struct DataResponse<T> {
    pub data: Vec<T>,
}

/// Search results come under `results.{type}`.
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub results: SearchResults,
}

#[derive(Debug, Deserialize)]
pub struct SearchResults {
    pub artists: Option<SearchResultGroup<ArtistResource>>,
    pub songs: Option<SearchResultGroup<SongResource>>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResultGroup<T> {
    pub data: Vec<T>,
    pub next: Option<String>,
}

/// View response for similar-artists and top-songs.
#[derive(Debug, Deserialize)]
pub struct ViewResponse<T> {
    pub data: Vec<T>,
    pub next: Option<String>,
}

// -- Library playlist types -------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LibraryPlaylistAttributes {
    pub name: String,
    pub description: Option<LibraryPlaylistDescription>,
    #[serde(rename = "canEdit")]
    pub can_edit: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryPlaylistDescription {
    pub standard: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryPlaylistResource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    pub attributes: Option<LibraryPlaylistAttributes>,
}

// -- Apple Music error body -------------------------------------------------

#[derive(Debug, Deserialize)]
struct AppleErrorResponse {
    errors: Vec<AppleError>,
}

#[derive(Debug, Deserialize)]
struct AppleError {
    status: String,
    title: String,
    detail: Option<String>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct AppleMusicClient {
    http: Client,
    /// ISO 3166-1 alpha-2 storefront code (e.g. "us").
    pub storefront: String,
}

impl AppleMusicClient {
    pub fn new(http: Client, storefront: String) -> Self {
        Self { http, storefront }
    }

    // -- Catalog Search -----------------------------------------------------

    /// `GET /v1/catalog/{storefront}/search?types=artists&term={term}&limit={limit}`
    ///
    /// Search the catalog for artists.
    /// Ref: https://developer.apple.com/documentation/applemusicapi/search_for_catalog_resources
    pub async fn search_artists(
        &self,
        developer_token: &str,
        term: &str,
        limit: u8,
    ) -> Result<Vec<ArtistResource>, AppError> {
        let url = format!(
            "{BASE}/v1/catalog/{storefront}/search?types=artists&term={term}&limit={limit}",
            storefront = self.storefront,
            term = url_encode(term),
        );
        let resp: SearchResponse = self.get_json(developer_token, None, &url).await?;
        Ok(resp.results.artists.map(|a| a.data).unwrap_or_default())
    }

    /// `GET /v1/catalog/{storefront}/search?types=songs&term={term}&limit={limit}`
    ///
    /// Search the catalog for songs.
    pub async fn search_songs(
        &self,
        developer_token: &str,
        term: &str,
        limit: u8,
    ) -> Result<Vec<SongResource>, AppError> {
        let url = format!(
            "{BASE}/v1/catalog/{storefront}/search?types=songs&term={term}&limit={limit}",
            storefront = self.storefront,
            term = url_encode(term),
        );
        let resp: SearchResponse = self.get_json(developer_token, None, &url).await?;
        Ok(resp.results.songs.map(|s| s.data).unwrap_or_default())
    }

    // -- Catalog Artist -----------------------------------------------------

    /// `GET /v1/catalog/{storefront}/artists/{id}`
    ///
    /// Fetch a single artist by ID.
    /// Ref: https://developer.apple.com/documentation/applemusicapi/get-a-catalog-artist
    pub async fn get_artist(
        &self,
        developer_token: &str,
        artist_id: &str,
    ) -> Result<ArtistResource, AppError> {
        let url = format!(
            "{BASE}/v1/catalog/{storefront}/artists/{artist_id}",
            storefront = self.storefront,
        );
        let resp: DataResponse<ArtistResource> = self.get_json(developer_token, None, &url).await?;
        resp.data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ArtistNotFound(artist_id.to_string()))
    }

    /// `GET /v1/catalog/{storefront}/artists/{id}/view/similar-artists`
    ///
    /// Fetch artists similar to the given artist via the `similar-artists` view.
    /// Ref: https://developer.apple.com/documentation/applemusicapi/artists/views/artistssimilarartistsview
    pub async fn get_similar_artists(
        &self,
        developer_token: &str,
        artist_id: &str,
    ) -> Result<Vec<ArtistResource>, AppError> {
        let url = format!(
            "{BASE}/v1/catalog/{storefront}/artists/{artist_id}/view/similar-artists",
            storefront = self.storefront,
        );
        let resp: ViewResponse<ArtistResource> = self.get_json(developer_token, None, &url).await?;
        Ok(resp.data)
    }

    /// `GET /v1/catalog/{storefront}/artists/{id}/view/top-songs`
    ///
    /// Fetch an artist's top songs via the `top-songs` view.
    /// Ref: https://developer.apple.com/documentation/applemusicapi/artists/views-data.dictionary
    pub async fn get_artist_top_songs(
        &self,
        developer_token: &str,
        artist_id: &str,
        limit: u8,
    ) -> Result<Vec<SongResource>, AppError> {
        let url = format!(
            "{BASE}/v1/catalog/{storefront}/artists/{artist_id}/view/top-songs?limit={limit}",
            storefront = self.storefront,
        );
        let resp: ViewResponse<SongResource> = self.get_json(developer_token, None, &url).await?;
        Ok(resp.data)
    }

    // -- Library Playlists (require Music User Token) -----------------------

    /// `POST /v1/me/library/playlists`
    ///
    /// Create a new library playlist.
    /// Requires: `Music-User-Token` header.
    /// Ref: https://developer.apple.com/documentation/applemusicapi/create-a-new-library-playlist
    pub async fn create_library_playlist(
        &self,
        developer_token: &str,
        user_token: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<LibraryPlaylistResource, AppError> {
        let body = serde_json::json!({
            "attributes": {
                "name": name,
                "description": description.unwrap_or(""),
            }
        });

        let url = format!("{BASE}/v1/me/library/playlists");
        let resp = self
            .post_json(developer_token, Some(user_token), &url, &body)
            .await?;

        // Response is 201 Created with `{ "data": [...] }`.
        let wrapper: DataResponse<LibraryPlaylistResource> =
            resp.json().await.map_err(AppError::from)?;

        wrapper
            .data
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Internal("Empty response from create playlist".into()))
    }

    /// `POST /v1/me/library/playlists/{id}/tracks`
    ///
    /// Add tracks to an existing library playlist.
    /// Body: `LibraryPlaylistTracksRequest` — `{ "data": [{ "id": "...", "type": "songs" }, ...] }`
    /// Requires: `Music-User-Token` header.
    /// Ref: https://developer.apple.com/documentation/applemusicapi/add-tracks-to-a-library-playlist
    pub async fn add_tracks_to_playlist(
        &self,
        developer_token: &str,
        user_token: &str,
        playlist_id: &str,
        song_ids: &[String],
    ) -> Result<(), AppError> {
        let data: Vec<serde_json::Value> = song_ids
            .iter()
            .map(|id| {
                serde_json::json!({
                    "id": id,
                    "type": "songs"
                })
            })
            .collect();

        let body = serde_json::json!({ "data": data });

        let url = format!("{BASE}/v1/me/library/playlists/{playlist_id}/tracks");
        let resp = self
            .post_json(developer_token, Some(user_token), &url, &body)
            .await?;

        // 204 No Content on success.
        if resp.status() == StatusCode::NO_CONTENT || resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(AppError::AppleMusicApi {
                status,
                message: text,
            })
        }
    }

    // -- Internal helpers ---------------------------------------------------

    /// Map a non-2xx Apple Music response body to an `AppError`, extracting
    /// the structured error message when present.
    fn map_error(status: StatusCode, text: String) -> AppError {
        let message = serde_json::from_str::<AppleErrorResponse>(&text)
            .ok()
            .and_then(|e| {
                e.errors
                    .into_iter()
                    .next()
                    .map(|err| err.detail.unwrap_or(err.title))
            })
            .unwrap_or(text);
        AppError::AppleMusicApi { status, message }
    }

    /// GET with exponential back-off on 429.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        developer_token: &str,
        user_token: Option<&str>,
        url: &str,
    ) -> Result<T, AppError> {
        let resp = request_with_backoff(
            "apple_music",
            || {
                let mut req = self.http.get(url).bearer_auth(developer_token);
                if let Some(ut) = user_token {
                    req = req.header("Music-User-Token", ut);
                }
                req
            },
            Self::map_error,
        )
        .await?;

        resp.json::<T>().await.map_err(AppError::from)
    }

    /// POST JSON with back-off on 429.
    async fn post_json(
        &self,
        developer_token: &str,
        user_token: Option<&str>,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, AppError> {
        request_with_backoff(
            "apple_music",
            || {
                let mut req = self.http.post(url).bearer_auth(developer_token).json(body);
                if let Some(ut) = user_token {
                    req = req.header("Music-User-Token", ut);
                }
                req
            },
            Self::map_error,
        )
        .await
    }
}
