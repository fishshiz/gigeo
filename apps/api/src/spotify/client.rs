//! Spotify Web API client.
//!
//! Endpoint references (OpenAPI spec):
//!   https://developer.spotify.com/reference/web-api/open-api-schema.yaml
//!
//! All endpoint paths, field names, and response shapes are taken directly
//! from the spec. Deprecated endpoints are avoided where alternatives exist.

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::http_utils::{request_with_backoff, url_encode};
use unicode_normalization::UnicodeNormalization;

/// Lowercases, strips accents (NFD), and drops non-alphanumeric characters —
/// used both to match a searched artist name against Spotify search results
/// and, exact-normalized, to match a Spotify top artist against a
/// Ticketmaster attraction name for personalized discovery.
pub(crate) fn normalize_artist_name(s: &str) -> String {
    s.to_lowercase()
        .nfd()
        .filter(|c| c.is_alphanumeric() && !c.is_whitespace())
        .collect()
}

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
    /// Absent from search-result items in practice but present on a full
    /// `GET /artists/{id}` response — defaulted rather than required so
    /// both shapes deserialize into the same struct.
    #[serde(default)]
    pub genres: Vec<String>,
    /// 0-100, present on both search-result and full artist objects
    /// (unlike `genres`) — defaulted for the same "both shapes deserialize"
    /// reasoning anyway, in case a future endpoint ever omits it.
    #[serde(default)]
    pub popularity: u32,
}

/// One release from `GET /artists/{id}/albums` — see `Client::get_artist_albums`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplifiedAlbum {
    pub id: String,
    pub name: String,
    /// `album_type` is one of `"album"`, `"single"`, `"compilation"` --
    /// filtered to the first two by `get_artist_albums`'s `include_groups`
    /// param, so a caller never sees `"compilation"` in practice, but the
    /// field is still modeled since it's always present on the wire.
    pub album_type: String,
    /// ISO 8601, but only as precise as `release_date_precision` says --
    /// `"year"` (`"1999"`), `"month"` (`"1999-12"`), or `"day"`
    /// (`"1999-12-31"`). Kept as the raw string rather than parsed here;
    /// `artists::worker::backfill_release_info` handles the precision
    /// split when deciding whether a release counts as recent.
    pub release_date: String,
    pub release_date_precision: String,
    #[serde(default)]
    pub images: Vec<Image>,
    pub external_urls: ExternalUrls,
}

#[derive(Debug, Deserialize)]
pub struct ArtistAlbumsResponse {
    pub items: Vec<SimplifiedAlbum>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimplifiedArtist {
    pub id: String,
    pub name: String,
}

/// Window over which Spotify computes "top artists" — passed as the
/// `time_range` query param on `GET /me/top/artists`.
#[derive(Debug, Clone, Copy)]
pub enum TopArtistsTimeRange {
    /// ~4 weeks
    Short,
    /// ~6 months
    Medium,
    /// Several years, calculated periodically
    Long,
}

impl TopArtistsTimeRange {
    fn as_str(self) -> &'static str {
        match self {
            Self::Short => "short_term",
            Self::Medium => "medium_term",
            Self::Long => "long_term",
        }
    }
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
pub struct User {
    pub id: String,
}

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
pub struct FollowedArtistsResponse {
    pub artists: FollowedArtistsPage,
}

/// `GET /me/following?type=artist`'s cursor-paginated shape -- distinct
/// from `Paging<T>`'s offset-based one used elsewhere in this client.
/// Only `items` is read; a single page (see `get_followed_artists`'s own
/// `limit`) is enough for personalization seeding, so `cursors`/`next`
/// aren't modeled.
#[derive(Debug, Deserialize)]
pub struct FollowedArtistsPage {
    pub items: Vec<Artist>,
}

#[derive(Debug, Deserialize)]
pub struct ArtistTracksResponse {
    pub tracks: Vec<Track>,
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
    pub owner: SpotifyUser,
    pub external_urls: ExternalUrls,
    pub snapshot_id: String,
    /// What Spotify actually stored, echoed back on creation — only
    /// controls whether the playlist shows on the owner's profile and in
    /// search, *not* the separate "Private" access-control toggle
    /// Spotify's own apps expose (that has no Web API equivalent). Worth
    /// comparing against what was requested rather than assuming it match.
    pub public: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyUser {
    pub external_urls: ExternalUrls,
    pub href: String,
    pub id: String,
    pub uri: String,
    pub display_name: Option<String>,
    #[serde(rename = "type")]
    #[serde(default = "SpotifyUser::default")]
    #[serde(skip_deserializing)]
    type_: String,
}

impl SpotifyUser {
    fn default() -> String {
        "user".into()
    }
}

#[derive(Debug, Deserialize)]
pub struct SnapshotResponse {
    pub snapshot_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaylistTracksInfo {
    pub total: u32,
}

#[derive(Debug, Deserialize)]
struct PlaylistTrackItem {
    track: Option<PlaylistItemTrack>,
}

#[derive(Debug, Deserialize)]
struct PlaylistItemTrack {
    uri: Option<String>,
}

/// A subset of the full playlist object, requested via the Web API's
/// `fields` filter so we don't pull down every track on every list refresh.
#[derive(Debug, Deserialize)]
pub struct PlaylistDetails {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub images: Vec<Image>,
    pub external_urls: ExternalUrls,
    pub tracks: PlaylistTracksInfo,
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
            url_encode(name),
        );
        self.get_json::<SearchArtistsResponse>(token, &url)
            .await
            .map(|r| r.artists.items)
    }

    /// Finds the best-matching artist for `artist_name` via
    /// `GET /search?q={name}&type=artist`, then returns up to `limit` of
    /// their top tracks via `GET /artists/{id}/top-tracks` — non-deprecated.
    /// The top-tracks endpoint has no `limit` param of its own and returns
    /// its full list regardless, so `limit` is enforced client-side here.
    pub async fn search_tracks_by_artist(
        &self,
        token: &str,
        artist_name: &str,
        limit: u8,
    ) -> Result<Option<Vec<Track>>, AppError> {
        let query = artist_name.to_string();
        let artist_url = format!(
            "{BASE}/search?q={}&type=artist&limit={limit}",
            url_encode(&query),
        );
        let artist = self
            .get_json::<SearchArtistsResponse>(token, &artist_url)
            .await?
            .artists
            .items
            .into_iter()
            .find(|a| normalize_artist_name(&a.name) == normalize_artist_name(artist_name));
        let Some(artist) = artist else {
            return Ok(None);
        };
        let tracks_url = format!("{BASE}/artists/{}/top-tracks", artist.id);
        self.get_json::<ArtistTracksResponse>(token, &tracks_url)
            .await
            .map(|r| Some(r.tracks.into_iter().take(limit as usize).collect()))
    }

    // -- Artists ------------------------------------------------------------

    /// `GET /artists/{id}`  — non-deprecated.
    pub async fn get_artist(&self, token: &str, id: &str) -> Result<Artist, AppError> {
        let url = format!("{BASE}/artists/{id}");
        self.get_json(token, &url).await
    }

    /// `GET /artists/{id}`  — non-deprecated.
    pub async fn get_artist_by_href(&self, token: &str, href: &str) -> Result<Artist, AppError> {
        let url = href.to_string();
        self.get_json(token, &url).await
    }

    /// `GET /artists/{id}/albums?include_groups=album,single&limit=10` --
    /// non-deprecated. Restricted to albums/singles (excludes
    /// compilations and appears-on credits) since those are what "new
    /// music" means for `artists::worker::backfill_release_info`'s recency
    /// check. The endpoint has no reliable sort guarantee, so the caller
    /// sorts by `release_date` itself -- 10 is comfortably more than any
    /// artist releases in a 30-day recency window, short of pulling every
    /// release they've ever put out.
    pub async fn get_artist_albums(
        &self,
        token: &str,
        id: &str,
    ) -> Result<Vec<SimplifiedAlbum>, AppError> {
        let url = format!("{BASE}/artists/{id}/albums?include_groups=album,single&limit=10");
        self.get_json::<ArtistAlbumsResponse>(token, &url)
            .await
            .map(|r| r.items)
    }

    /// `GET /me/top/artists?time_range={time_range}&limit={limit}`
    /// Scopes: `user-top-read`
    pub async fn get_top_artists(
        &self,
        user_token: &str,
        time_range: TopArtistsTimeRange,
        limit: u8,
    ) -> Result<Vec<Artist>, AppError> {
        let url = format!(
            "{BASE}/me/top/artists?time_range={}&limit={limit}",
            time_range.as_str(),
        );
        self.get_json::<Paging<Artist>>(user_token, &url)
            .await
            .map(|p| p.items)
    }

    /// `GET /me/following?type=artist&limit={limit}` — non-deprecated.
    /// Scopes: `user-follow-read`. First page only, same "good enough for
    /// a seed pool" reasoning `get_top_artists` already applies to its own
    /// 50-artist cap — see `spotify::top_artists`.
    pub async fn get_followed_artists(
        &self,
        user_token: &str,
        limit: u8,
    ) -> Result<Vec<Artist>, AppError> {
        let url = format!("{BASE}/me/following?type=artist&limit={limit}");
        self.get_json::<FollowedArtistsResponse>(user_token, &url)
            .await
            .map(|r| r.artists.items)
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

    /// `PUT /playlists/{playlist_id}`
    /// Updates a playlist's name, public visibility, and/or description.
    /// No-ops (without making a request) if `name`, `public`, and
    /// `description` are all `None`.
    /// Scopes: `playlist-modify-public`, `playlist-modify-private`
    pub async fn update_playlist_details(
        &self,
        user_token: &str,
        playlist_id: &str,
        name: Option<&str>,
        public: Option<bool>,
        description: Option<&str>,
    ) -> Result<(), AppError> {
        let body = Self::build_update_details_body(name, public, description);
        if body.as_object().is_some_and(|m| m.is_empty()) {
            return Ok(());
        }

        self.put_with_backoff(
            &format!("{BASE}/playlists/{playlist_id}"),
            user_token,
            &body,
        )
        .await?;

        Ok(())
    }

    fn build_update_details_body(
        name: Option<&str>,
        public: Option<bool>,
        description: Option<&str>,
    ) -> serde_json::Value {
        let mut body = serde_json::Map::new();
        if let Some(n) = name {
            body.insert("name".into(), serde_json::json!(n));
        }
        if let Some(p) = public {
            body.insert("public".into(), serde_json::json!(p));
        }
        if let Some(d) = description {
            body.insert("description".into(), serde_json::json!(d));
        }
        serde_json::Value::Object(body)
    }

    /// `DELETE /playlists/{playlist_id}/followers`
    /// Spotify's Web API has no true "delete playlist" endpoint — unfollowing
    /// is the closest primitive, and for the owner it removes the playlist
    /// from their library.
    /// Scopes: `playlist-modify-public`, `playlist-modify-private`
    pub async fn unfollow_playlist(
        &self,
        user_token: &str,
        playlist_id: &str,
    ) -> Result<(), AppError> {
        self.delete_with_backoff(
            &format!("{BASE}/playlists/{playlist_id}/followers"),
            user_token,
        )
        .await?;
        Ok(())
    }

    /// `GET /playlists/{playlist_id}` — non-deprecated.
    /// Uses `fields` to fetch only what the "My Playlists" list needs.
    pub async fn get_playlist(
        &self,
        token: &str,
        playlist_id: &str,
    ) -> Result<PlaylistDetails, AppError> {
        let url = format!(
            "{BASE}/playlists/{playlist_id}?fields=id,name,images,external_urls,tracks.total"
        );
        self.get_json(token, &url).await
    }

    /// `GET /playlists/{playlist_id}/tracks` — non-deprecated.
    /// Paginates via Spotify's `next` URL until exhausted, returning just
    /// the track URIs. Skips local-file/unavailable tracks (null `track`
    /// or `uri`). Used by the periodic updater to diff current vs.
    /// newly-found tracks.
    pub async fn get_playlist_tracks(
        &self,
        token: &str,
        playlist_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let mut uris = Vec::new();
        let mut url = format!(
            "{BASE}/playlists/{playlist_id}/tracks?fields=items(track(uri)),next,limit,offset,total&limit=100"
        );

        loop {
            let page: Paging<PlaylistTrackItem> = self.get_json(token, &url).await?;
            uris.extend(
                page.items
                    .into_iter()
                    .filter_map(|i| i.track.and_then(|t| t.uri)),
            );

            match page.next {
                Some(next_url) => url = next_url,
                None => break,
            }
        }

        Ok(uris)
    }

    // -- Internal helpers ---------------------------------------------------

    /// Map a non-2xx Spotify response body to an `AppError`, extracting the
    /// structured error message when present.
    fn map_error(status: reqwest::StatusCode, text: String) -> AppError {
        let message = serde_json::from_str::<SpotifyErrorWrapper>(&text)
            .map(|e| e.error.message)
            .unwrap_or(text);
        AppError::SpotifyApi { status, message }
    }

    /// GET a URL, parse JSON, with retry on 429.
    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        token: &str,
        url: &str,
    ) -> Result<T, AppError> {
        let resp = request_with_backoff(
            "spotify",
            || self.http.get(url).bearer_auth(token),
            Self::map_error,
        )
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
        request_with_backoff(
            "spotify",
            || self.http.post(url).bearer_auth(token).json(body),
            Self::map_error,
        )
        .await
    }

    /// PUT JSON with retry on 429.
    async fn put_with_backoff(
        &self,
        url: &str,
        token: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::Response, AppError> {
        request_with_backoff(
            "spotify",
            || self.http.put(url).bearer_auth(token).json(body),
            Self::map_error,
        )
        .await
    }

    /// DELETE with retry on 429, no body.
    async fn delete_with_backoff(
        &self,
        url: &str,
        token: &str,
    ) -> Result<reqwest::Response, AppError> {
        request_with_backoff(
            "spotify",
            || self.http.delete(url).bearer_auth(token),
            Self::map_error,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_track_page_parses_and_filters_unavailable_tracks() {
        let json = r#"
        {
            "items": [
                { "track": { "uri": "spotify:track:1" } },
                { "track": { "uri": "spotify:track:2" } },
                { "track": null },
                { "track": { "uri": null } }
            ],
            "next": "https://api.spotify.com/v1/playlists/abc/tracks?offset=100",
            "limit": 100,
            "offset": 0,
            "total": 4
        }
        "#;

        let page: Paging<PlaylistTrackItem> = serde_json::from_str(json).unwrap();
        let uris: Vec<String> = page
            .items
            .into_iter()
            .filter_map(|i| i.track.and_then(|t| t.uri))
            .collect();

        assert_eq!(uris, vec!["spotify:track:1", "spotify:track:2"]);
        assert_eq!(
            page.next.as_deref(),
            Some("https://api.spotify.com/v1/playlists/abc/tracks?offset=100")
        );
    }

    #[test]
    fn playlist_track_page_with_no_next_terminates_pagination() {
        let json = r#"{"items": [], "limit": 100, "offset": 100, "total": 4}"#;
        let page: Paging<PlaylistTrackItem> = serde_json::from_str(json).unwrap();
        assert!(page.next.is_none());
        assert!(page.items.is_empty());
    }

    #[test]
    fn build_update_details_body_empty_when_nothing_provided() {
        let body = SpotifyClient::build_update_details_body(None, None, None);
        assert_eq!(body, serde_json::json!({}));
    }

    #[test]
    fn build_update_details_body_includes_only_name() {
        let body = SpotifyClient::build_update_details_body(Some("New Name"), None, None);
        assert_eq!(body, serde_json::json!({ "name": "New Name" }));
    }

    #[test]
    fn build_update_details_body_includes_only_public() {
        let body = SpotifyClient::build_update_details_body(None, Some(true), None);
        assert_eq!(body, serde_json::json!({ "public": true }));
    }

    #[test]
    fn build_update_details_body_includes_only_description() {
        let body = SpotifyClient::build_update_details_body(None, None, Some("• Artist — Aug 20"));
        assert_eq!(
            body,
            serde_json::json!({ "description": "• Artist — Aug 20" })
        );
    }

    #[test]
    fn build_update_details_body_includes_all() {
        let body =
            SpotifyClient::build_update_details_body(Some("New Name"), Some(false), Some("desc"));
        assert_eq!(
            body,
            serde_json::json!({ "name": "New Name", "public": false, "description": "desc" })
        );
    }
}
