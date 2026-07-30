use crate::error::AppError;
use crate::spotify::client::{Artist, Image};
use crate::state::AppState;
use axum::{Json, extract::State};
use axum_extra::extract::Query as QueryArray;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// GET /artist?name=<artist_name>&name=<artist_name>&...
// ---------------------------------------------------------------------------

/// Accepts one or more `name` query params:
///   GET /artist?name=Diana+Krall
///   GET /artist?name=Radiohead&name=Bjork&name=Aphex+Twin
///
/// Uses `axum_extra::extract::Query` which handles repeated keys
/// natively (backed by `serde_html_form` instead of `serde_urlencoded`).
#[derive(Deserialize)]
pub struct ArtistQuery {
    pub name: Vec<String>,
}

#[derive(Serialize)]
pub struct ArtistInfoResponse {
    pub name: String,
    pub id: String,
    pub uri: String,
    pub spotify_url: Option<String>,
    pub images: Vec<Image>,
}

/// Look up one or more artists by name and return their info
///
/// Uses Client Credentials auth (no user login required).
///
/// Spotify endpoints used:
///   - `GET /search` (type=artist) — non-deprecated
pub async fn get_artist_info(
    State(state): State<AppState>,
    QueryArray(q): QueryArray<ArtistQuery>,
) -> Result<Json<Vec<ArtistInfoResponse>>, AppError> {
    if q.name.is_empty() {
        return Err(AppError::Internal(
            "At least one `name` query parameter is required".into(),
        ));
    }

    let token = state.cc_manager.get_token().await?;
    let mut results = Vec::with_capacity(q.name.len());

    for artist_name in &q.name {
        // Search for the artist by name.
        let search_results: Vec<Artist> = state
            .spotify
            .search_artist(&token.access_token, artist_name, 1)
            .await?;
        let artist = search_results
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ArtistNotFound(artist_name.clone()))?;

        results.push(ArtistInfoResponse {
            name: artist.name,
            id: artist.id,
            uri: artist.uri,
            spotify_url: artist.external_urls.spotify,
            images: artist.images,
        });
    }

    Ok(Json(results))
}
