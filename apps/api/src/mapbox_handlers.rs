use crate::state::AppState;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::{Json, http::StatusCode};
use geojson::FeatureCollection;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct CitiesQuery {
    q: Option<String>,
}

pub async fn get_cities(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CitiesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let query = params.q.unwrap_or_default();

    let url = format!(
        "https://api.mapbox.com/search/geocode/v6/forward?q={}&types=place&access_token={}",
        query, state.mapbox_key
    );

    let resp = state.client.get(&url).send().await.map_err(|e| {
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

    let body: FeatureCollection = serde_json::from_str(&text).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Failed to decode mapbox body: {}", e),
        )
    })?;

    Ok(Json(body))
}
