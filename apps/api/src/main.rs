mod apple_music;
mod apple_music_updater_handlers;
mod auth;
mod config;
mod error;
mod spotify;
mod spotify_handlers;
mod state;
mod updater;

use std::sync::Arc;

use geohash::{encode, Coord};
use tower_http::cors::CorsLayer;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::config::AppConfig;
use auth::{ClientCredentialsManager, SpotifyCredentials, UserTokenManager};
use reqwest::Client;
use spotify::SpotifyClient;
use state::AppState;
use tracing::info;
use updater::ArtistQueue;

use apple_music::auth::{AppleMusicCredentials, DeveloperTokenManager, MusicUserTokenStore};
use apple_music::client::AppleMusicClient;
use apple_music::updater::AppleArtistQueue;

use geojson::FeatureCollection;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present (ignore errors if missing).
    let _ = dotenvy::dotenv();

    // Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load credentials from environment.
    let creds = SpotifyCredentials::from_env();
    let http = reqwest::Client::new();

    let cc_manager = ClientCredentialsManager::new(creds.clone(), http.clone());
    let user_manager = UserTokenManager::new(creds.clone(), http.clone());
    let spotify = SpotifyClient::new(http.clone());
    let artist_queue = ArtistQueue::new();

    // Load configuration.
    let config = AppConfig::from_env()?;

    // Extract mapbox key before moving `config` into the worker.
    let mapbox_key_clone = config.mapbox_private_key.clone();

    // Extract ticketmaster key before moving `config` into the worker.
    let ticketmaster_key_clone = config.ticketmaster_api_key.clone();

    let client = Client::new();

    // -- Apple Music setup (optional — skip if env vars aren't set) ---------
    let (apple_client, apple_dev, apple_user, apple_queue) = if std::env::var("APPLE_MUSIC_TEAM_ID")
        .is_ok()
    {
        let creds = AppleMusicCredentials::from_env();
        let storefront = std::env::var("APPLE_MUSIC_STOREFRONT").unwrap_or_else(|_| "us".into());
        let client = AppleMusicClient::new(http.clone(), storefront);
        let dev = DeveloperTokenManager::new(creds);
        let user = MusicUserTokenStore::new();
        let queue = AppleArtistQueue::new();
        (Some(client), Some(dev), Some(user), Some(queue))
    } else {
        tracing::info!("Apple Music env vars not set — Apple Music routes will return errors");
        (None, None, None, None)
    };

    // Build the Axum router.
    let shared_state = Arc::new(AppState {
        client,
        mapbox_key: mapbox_key_clone,
        ticketmaster_key: ticketmaster_key_clone,
        cc_manager,
        user_manager,
        spotify,
        artist_queue,
        apple_music_client: apple_client,
        apple_dev_token: apple_dev,
        apple_user_token: apple_user,
        apple_artist_queue: apple_queue.clone(),
    });
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/cities", get(get_cities))
        .route("/concerts", get(get_concerts_tm))
        .route("/artists", get(spotify_handlers::get_artist_info))
        // Apple Music routes
        .route("/apple/artist", get(apple_music::handlers::get_artist_info))
        .route(
            "/apple/playlist",
            post(apple_music::handlers::create_playlist),
        )
        .route(
            "/apple/user-token",
            post(apple_music::handlers::set_user_token),
        )
        .route(
            "/apple/updater/config",
            post(apple_music_updater_handlers::configure_apple_updater),
        )
        .route(
            "/apple/updater/artists",
            put(apple_music_updater_handlers::update_apple_artists),
        )
        .layer(CorsLayer::permissive())
        .with_state(shared_state);

    // Start the HTTP server.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .context("Failed to bind to port 3000")?;

    info!("HTTP server listening on 0.0.0.0:3000");
    axum::serve(listener, app)
        .await
        .context("HTTP server error")?;

    Ok(())
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}
#[derive(Deserialize)]
struct CitiesQuery {
    q: Option<String>,
}

async fn get_cities(
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

#[derive(Deserialize)]
struct EventsQuery {
    latitude: f64,
    longitude: f64,
    radius: u8,
    start: String,
    end: String,
}

#[derive(Debug, Deserialize)]
struct TicketmasterResponse {
    #[serde(rename = "_embedded")]
    embedded: Option<Embedded>,
}

#[derive(Debug, Deserialize)]
struct Embedded {
    #[serde(default)]
    events: Vec<TmEvent>,
}

#[derive(Debug, Deserialize)]
struct TmEvent {
    id: String,
    name: String,
    #[serde(default)]
    images: Vec<Images>,
    dates: TmDate,
    #[serde(rename = "_embedded")]
    embedded: Option<EventEmbedded>,
}

#[derive(Debug, Deserialize)]
struct TmDate {
    start: TmDateStart,
}

#[derive(Debug, Deserialize)]
struct TmDateStart {
    // Ticketmaster may omit this for some events
    #[serde(default)]
    dateTime: Option<String>,
    #[serde(default)]
    localDate: Option<String>,
    #[serde(default)]
    localTime: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventEmbedded {
    venues: Option<Vec<TmVenue>>,
    attractions: Option<Vec<TmAttraction>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmAttraction {
    name: Option<String>,
    classifications: Option<Vec<TmClassification>>,
    externalLinks: Option<TmExternalLinks>,
    images: Option<Vec<Images>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmExternalLinks {
    wiki: Option<Vec<TmExternalLink>>,
    homepage: Option<Vec<TmExternalLink>>,
    instagram: Option<Vec<TmExternalLink>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmExternalLink {
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmClassification {
    primary: bool,
    segment: Option<TmSegment>,
    genre: Option<TmSegment>,
    subGenre: Option<TmSegment>,
    subType: Option<TmSegment>,
    family: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TmSegment {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct TmVenue {
    name: Option<String>,
    location: Option<TmLocation>,
}

#[derive(Debug, Deserialize, Clone)]
struct TmLocation {
    latitude: Option<String>,
    longitude: Option<String>,
}

#[derive(Debug, Serialize)]
struct EventResponse {
    id: String,
    name: String,
    venue: Option<VenueResponse>,
    images: Vec<Images>,
    dates: Option<String>,
    attractions: Option<Vec<TmAttraction>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Attraction {
    name: Option<String>,
    classifications: Option<Vec<TmClassification>>,
    externalLinks: Option<TmExternalLinks>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Images {
    ratio: Option<String>,
    url: String,
    width: Option<i32>,
    height: Option<i32>,
    fallback: Option<bool>,
}

#[derive(Debug, Serialize)]
struct VenueResponse {
    name: Option<String>,
    location: Option<LocationResponse>,
}

#[derive(Debug, Serialize)]
struct LocationResponse {
    latitude: Option<String>,
    longitude: Option<String>,
}

async fn get_concerts_tm(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventsQuery>,
) -> Result<Json<Vec<EventResponse>>, (axum::http::StatusCode, String)> {
    let latitude: f64 = params.latitude;
    let longitude: f64 = params.longitude;
    let radius = params.radius;

    let hash = encode(
        Coord {
            x: longitude,
            y: latitude,
        },
        6usize,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode geohash: {}", e),
        )
    })?;

    let start = params.start;
    let end = params.end;
    let url = format!(
        "https://app.ticketmaster.com/discovery/v2/events.json?geoPoint={}&apikey={}&radius={}&startDateTime={}&endDateTime={}&size=200&sort=date,asc",
        hash, state.ticketmaster_key, radius, start, end
    );
    println!("{}", &url);
    let resp = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| (axum::http::StatusCode::NOT_FOUND, e.to_string()))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("error reading body: {e}")))?;

    println!("TM status = {status}, body = {text}");

    let body: TicketmasterResponse = serde_json::from_str(&text)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("serde error: {e:?}")))?;

    // 2) project into your flattened DTO
    let events = body
        .embedded
        .into_iter()
        .flat_map(|e| e.events)
        .map(|e| {
            // choose dateTime if present, else synthesize from localDate/localTime, else None
            let dates = e.dates.start.dateTime.or_else(|| {
                match (
                    e.dates.start.localDate.as_ref(),
                    e.dates.start.localTime.as_ref(),
                ) {
                    (Some(d), Some(t)) => Some(format!("{d}T{t}")),
                    (Some(d), None) => Some(d.clone()),
                    _ => None,
                }
            });
            let venue = e
                .embedded
                .as_ref()
                .and_then(|emb| emb.venues.as_ref())
                .and_then(|mut vs| vs.last().cloned()); // first venue

            let venue = venue.map(|v| VenueResponse {
                name: v.name,
                location: v.location.map(|loc| LocationResponse {
                    latitude: loc.latitude,
                    longitude: loc.longitude,
                }),
            });

            let attractions = e.embedded.and_then(|a| a.attractions);

            EventResponse {
                id: e.id,
                name: e.name,
                venue,
                images: e.images,
                dates,
                attractions,
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(events))
}
