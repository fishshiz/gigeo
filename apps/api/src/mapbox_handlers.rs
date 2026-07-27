use crate::http_utils::url_encode;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use geojson::FeatureCollection;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CitiesQuery {
    q: Option<String>,
}

#[derive(Deserialize)]
pub struct ReverseGeocodeQuery {
    longitude: f64,
    latitude: f64,
}

async fn fetch_mapbox_features(
    state: &AppState,
    url: &str,
) -> Result<FeatureCollection, (StatusCode, String)> {
    let resp = state.client.get(url).send().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to fetch from mapbox: {}", e),
        )
    })?;

    if !resp.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Mapbox returned status {}", resp.status()),
        ));
    }

    let text = resp.text().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to read mapbox body: {}", e),
        )
    })?;

    serde_json::from_str(&text).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to decode mapbox body: {}", e),
        )
    })
}

pub async fn get_cities(
    State(state): State<AppState>,
    Query(params): Query<CitiesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let query = params.q.unwrap_or_default();

    let url = format!(
        "https://api.mapbox.com/search/geocode/v6/forward?q={}&types=place&access_token={}",
        url_encode(&query),
        state.mapbox_key
    );

    let body = fetch_mapbox_features(&state, &url).await?;

    Ok(Json(body))
}

/// Reverse-geocodes coordinates (e.g. from the map's geolocate control)
/// into the nearest place name, in the same shape `get_cities` returns so
/// the frontend can treat a geolocated result identically to a searched one.
pub async fn reverse_geocode(
    State(state): State<AppState>,
    Query(params): Query<ReverseGeocodeQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let url = format!(
        "https://api.mapbox.com/search/geocode/v6/reverse?longitude={}&latitude={}&types=place&access_token={}",
        params.longitude, params.latitude, state.mapbox_key
    );

    let body = fetch_mapbox_features(&state, &url).await?;

    Ok(Json(body))
}
