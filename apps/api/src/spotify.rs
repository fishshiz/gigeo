//! Spotify Web API client.
//!
//! Endpoint references (OpenAPI spec):
//!   https://developer.spotify.com/reference/web-api/open-api-schema.yaml
//!
//! All endpoint paths, field names, and response shapes are taken directly
//! from the spec. Deprecated endpoints are avoided where alternatives exist.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const BASE: &str = "https://api.spotify.com/v1";

// ---------------------------------------------------------------------------
// Response / domain types (matching the OpenAPI schema field names)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Image {
    pub url: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalUrls {
    pub spotify: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub href: String,
    pub external_urls: ExternalUrls,
    #[serde(default)]
    pub images: Vec<Image>,
    #[serde(rename = "type")]
    pub artist_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplifiedArtist {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub duration_ms: u64,
    pub track_number: u32,
    pub disc_number: u32,
    pub explicit: bool,
    #[serde(default)]
    pub artists: Vec<SimplifiedArtist>,
}

// -- Search response wrappers -----------------------------------------------

#[derive(Debug, Deserialize)]
pub struct Paging<T> {
    pub items: Vec<T>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchArtistsResponse {
    pub artists: Paging<Artist>,
}

#[derive(Debug, Deserialize)]
pub struct SearchTracksResponse {
    pub tracks: Paging<Track>,
}

#[derive(Debug, Deserialize)]
pub struct RelatedArtistsResponse {
    pub artists: Vec<Artist>,
}

// -- Playlist types ---------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub uri: String,
    pub external_urls: ExternalUrls,
    pub snapshot_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SnapshotResponse {
    pub snapshot_id: String,
}

// -- Spotify error body -----------------------------------------------------

#[derive(Debug, Deserialize)]
struct SpotifyErrorWrapper {
    error: SpotifyErrorBody,
}

#[derive(Debug, Deserialize)]
struct SpotifyErrorBody {
    status: u16,
    message: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Thin wrapper that issues Spotify Web API requests.
/// Callers provide the access token; this struct handles
/// rate limiting (429 + Retry-After) via exponential back-off.
#[derive(Clone)]
pub struct SpotifyClient {
    http: Client,
}

impl SpotifyClient {
    pub fn new(http: Client) -> Self {
        Self { http }
    }

    // -- Search -------------------------------------------------------------

    /// `GET /search?q={query}&type=artist&limit={limit}`
    /// Non-deprecated. Used to look up an artist by name.
    pub async fn search_artist(
        &self,
        token: &str,
        name: &str,
        limit: u8,
    ) -> Result<Vec<Artist>, AppError> {
        let url = format!(
            "{BASE}/search?q={}&type=artist&limit={limit}",
            urlencoding(name),
        );
        self.get_json::<SearchArtistsResponse>(token, &url)
            .await
            .map(|r| r.artists.items)
    }

    /// `GET /search?q=artist:{artist_name}&type=track&limit={limit}`
    /// Non-deprecated. Returns tracks matching the given artist name.
    pub async fn search_tracks_by_artist(
        &self,
        token: &str,
        artist_name: &str,
        limit: u8,
    ) -> Result<Vec<Track>, AppError> {
        let query = format!("artist:{artist_name}");
        let url = format!(
            "{BASE}/search?q={}&type=track&limit={limit}",
            urlencoding(&query),
        );
        self.get_json::<SearchTracksResponse>(token, &url)
            .await
            .map(|r| r.tracks.items)
    }

    // -- Artists ------------------------------------------------------------

    /// `GET /artists/{id}`  — non-deprecated.
    pub async fn get_artist(&self, token: &str, id: &str) -> Result<Artist, AppError> {
        let url = format!("{BASE}/artists/{id}");
        self.get_json(token, &url).await
    }

    /// `GET /artists/{id}`  — non-deprecated.
    pub async fn get_artist_by_href(&self, token: &str, href: &str) -> Result<Artist, AppError> {
        let url = format!("{href}");
        self.get_json(token, &url).await
    }

    // -- Playlists (require user-scoped token) ------------------------------

    /// `POST /me/playlists`
    /// Scopes: `playlist-modify-public`, `playlist-modify-private`
    pub async fn create_playlist(
        &self,
        user_token: &str,
        name: &str,
        description: Option<&str>,
        public: bool,
    ) -> Result<Playlist, AppError> {
        let body = serde_json::json!({
            "name": name,
            "description": description.unwrap_or(""),
            "public": public,
        });

        let resp = self
            .post_with_backoff(&format!("{BASE}/me/playlists"), user_token, &body)
            .await?;

        resp.json::<Playlist>().await.map_err(AppError::from)
    }

    /// `POST /playlists/{playlist_id}/items`
    /// Adds tracks (by URI) to an existing playlist. Max 100 URIs per call.
    /// Scopes: `playlist-modify-public`, `playlist-modify-private`
    pub async fn add_items_to_playlist(
        &self,
        user_token: &str,
        playlist_id: &str,
        uris: &[String],
    ) -> Result<SnapshotResponse, AppError> {
        assert!(
            uris.len() <= 100,
            "add_items_to_playlist: max 100 URIs per call"
        );

        let body = serde_json::json!({ "uris": uris });

        let resp = self
            .post_with_backoff(
                &format!("{BASE}/playlists/{playlist_id}/items"),
                user_token,
                &body,
            )
            .await?;

        resp.json::<SnapshotResponse>()
            .await
            .map_err(AppError::from)
    }

    /// `PUT /playlists/{playlist_id}/items`
    /// Replaces all items in a playlist. Used by the periodic updater.
    /// Scopes: `playlist-modify-public`, `playlist-modify-private`
    pub async fn replace_playlist_items(
        &self,
        user_token: &str,
        playlist_id: &str,
        uris: &[String],
    ) -> Result<SnapshotResponse, AppError> {
        let body = serde_json::json!({ "uris": uris });

        let resp = self
            .put_with_backoff(
                &format!("{BASE}/playlists/{playlist_id}/items"),
                user_token,
                &body,
            )
            .await?;

        resp.json::<SnapshotResponse>()
            .await
            .map_err(AppError::from)
    }

    // -- Internal helpers ---------------------------------------------------

    /// GET a URL, parse JSON, with retry on 429.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        token: &str,
        url: &str,
    ) -> Result<T, AppError> {
        let resp = self
            .request_with_backoff(|| self.http.get(url).bearer_auth(token))
            .await?;
        resp.json::<T>().await.map_err(AppError::from)
    }

    /// POST JSON with retry on 429.
    async fn post_with_backoff(
        &self,
        url: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, AppError> {
        self.request_with_backoff(|| self.http.post(url).bearer_auth(token).json(body))
            .await
    }

    /// PUT JSON with retry on 429.
    async fn put_with_backoff(
        &self,
        url: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, AppError> {
        self.request_with_backoff(|| self.http.put(url).bearer_auth(token).json(body))
            .await
    }

    /// Execute a request builder with exponential back-off on 429.
    /// Respects the `Retry-After` header per Spotify rate-limit guidelines.
    async fn request_with_backoff(
        &self,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, AppError> {
        let max_retries: u32 = 5;
        let mut attempt = 0u32;

        loop {
            let resp = build().send().await?;

            if resp.status() == StatusCode::TOO_MANY_REQUESTS {
                attempt += 1;
                if attempt > max_retries {
                    let retry_after = parse_retry_after(&resp);
                    return Err(AppError::RateLimited {
                        retry_after_secs: retry_after,
                    });
                }

                let server_wait = parse_retry_after(&resp);
                let backoff = server_wait.max(1) * 2u64.pow(attempt - 1);
                tracing::warn!(attempt, backoff, "Rate limited (429), backing off");
                tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;
                continue;
            }

            // For non-429 errors, parse the Spotify error body.
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let text = resp.text().await.unwrap_or_default();

                // Try to extract the structured error message.
                let message = serde_json::from_str::<SpotifyErrorWrapper>(&text)
                    .map(|e| e.error.message)
                    .unwrap_or(text);

                return Err(AppError::SpotifyApi { status, message });
            }

            return Ok(resp);
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

fn parse_retry_after(resp: &reqwest::Response) -> u64 {
    resp.headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace('@', "%40")
}
