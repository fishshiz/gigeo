// src/lib.rs
mod apple_music;
mod auth;
mod config;
mod cookie;
mod db;
mod error;
mod http_utils;
mod mapbox_handlers;
mod services;
mod spotify;
mod state;
mod ticketmaster_handlers;
mod ticketmaster_stream;
mod token_cache;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, Method},
};
use reqwest::Client;
use serde_json::{Value, json};
use sqlx::migrate;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tracing_subscriber::EnvFilter;

use apple_music::auth::{AppleMusicCredentials, DeveloperTokenManager, MusicUserTokenStore};
use apple_music::client::AppleMusicClient;
use auth::{ClientCredentialsManager, SpotifyCredentials, UserTokenManager};
use axum_extra::extract::cookie::Key;

use config::AppConfig;
use db::AppDatabase;
use spotify::client::SpotifyClient;
use state::{AppState, AppStateInner};

pub async fn build_app() -> Result<Router> {
    // Load .env if present.
    let _ = dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Load credentials from env.
    let creds = SpotifyCredentials::from_env();
    let http = reqwest::Client::new();

    let cc_manager = ClientCredentialsManager::new(creds.clone(), http.clone());
    let user_manager = UserTokenManager::new(creds.clone(), http.clone());
    let spotify = SpotifyClient::new(http.clone());

    // DB connection
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file or environment");
    let db = AppDatabase::new(&database_url)
        .await
        .expect("Failed to connect to database");
    tracing::info!("connected to database");
    migrate!("./migrations")
        .run(&db.pool)
        .await
        .expect("Failed to run database migrations");

    // Load configuration.
    let config = AppConfig::from_env()?;

    let allowed_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .map(|origin| {
            origin
                .parse()
                .unwrap_or_else(|_| panic!("Invalid CORS origin: {origin}"))
        })
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    let mapbox_key_clone = config.mapbox_private_key.clone();
    let ticketmaster_key_clone = config.ticketmaster_api_key.clone();

    let client = Client::new();

    // Apple Music optional setup
    let (apple_client, apple_dev, apple_user) = if std::env::var("APPLE_MUSIC_TEAM_ID").is_ok() {
        let creds = AppleMusicCredentials::from_env();
        let storefront = std::env::var("APPLE_MUSIC_STOREFRONT").unwrap_or_else(|_| "us".into());
        let client = AppleMusicClient::new(http.clone(), storefront);
        let dev = DeveloperTokenManager::new(creds);
        let user = MusicUserTokenStore::new();
        (Some(client), Some(dev), Some(user))
    } else {
        tracing::info!("Apple Music env vars not set — Apple Music routes will return errors");
        (None, None, None)
    };

    let shared_state = AppState(Arc::new(AppStateInner {
        db,
        client,
        mapbox_key: mapbox_key_clone,
        ticketmaster_key: ticketmaster_key_clone,
        spotify,
        cc_manager,
        user_manager,
        spotify_client_id: std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID required"),
        spotify_client_secret: std::env::var("SPOTIFY_CLIENT_SECRET")
            .expect("SPOTIFY_CLIENT_SECRET required"),

        apple_music_client: apple_client,
        apple_dev_token: apple_dev,
        apple_user_token: apple_user,
        cookie_domain: config.cookie_domain.clone(),
        cookie_secure: config.cookie_secure,
        cookie_key: Key::from(config.cookie_key.as_bytes()),
    }));

    services::playlist_updater::spawn(shared_state.clone());

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "ok" }))
        .route("/health", axum::routing::get(health_check))
        .route(
            "/auth/status",
            axum::routing::get(spotify::spotify_handlers::auth_status),
        )
        .route("/cities", axum::routing::get(mapbox_handlers::get_cities))
        .route(
            "/reverse-geocode",
            axum::routing::get(mapbox_handlers::reverse_geocode),
        )
        .route(
            "/concerts/stream",
            axum::routing::get(ticketmaster_stream::get_concerts_tm_stream),
        )
        .route(
            "/future-events",
            axum::routing::get(ticketmaster_handlers::get_events_by_attraction),
        )
        .route(
            "/artists",
            axum::routing::get(spotify::spotify_handlers::get_artist_info),
        )
        .route(
            "/spotify/playlist",
            axum::routing::post(spotify::spotify_handlers::create_playlist),
        )
        .route(
            "/spotify/playlist",
            axum::routing::get(spotify::spotify_handlers::get_user_playlists),
        )
        .route(
            "/spotify/playlist/{id}",
            axum::routing::patch(spotify::spotify_handlers::update_playlist)
                .delete(spotify::spotify_handlers::delete_playlist),
        )
        .route(
            "/spotify/login",
            axum::routing::get(spotify::spotify_handlers::login),
        )
        .route(
            "/spotify/callback",
            axum::routing::get(spotify::spotify_handlers::oauth_callback),
        )
        .route(
            "/apple/artist",
            axum::routing::get(apple_music::handlers::get_artist_info),
        )
        .route(
            "/apple/playlist",
            axum::routing::post(apple_music::handlers::create_playlist),
        )
        .route(
            "/apple/user-token",
            axum::routing::post(apple_music::handlers::set_user_token),
        )
        .layer(cors)
        .with_state(shared_state);

    Ok(app)
}

pub async fn health_check(State(state): State<AppState>) -> Json<Value> {
    match sqlx::query("SELECT 1").execute(&state.db.pool).await {
        Ok(_) => Json(json!({
            "status": "ok",
            "database": "connected"
        })),
        Err(e) => {
            eprintln!("Database error: {}", e);
            Json(json!({
                "status": "error",
                "database": "disconnected",
                "error": e.to_string()
            }))
        }
    }
}
