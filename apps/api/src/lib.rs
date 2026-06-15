// src/lib.rs
mod apple_music;
mod apple_music_updater_handlers;
mod auth;
mod config;
mod cookie;
mod db;
mod error;
mod mapbox_handlers;
mod models;
mod spotify;
mod state;
mod ticketmaster_handlers;
mod ticketmaster_stream;
mod updater;

use std::sync::Arc;

use anyhow::Result;
use axum::{Json, Router, extract::State};
use reqwest::Client;
use serde_json::{Value, json};
use sqlx::migrate;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use apple_music::auth::{AppleMusicCredentials, DeveloperTokenManager, MusicUserTokenStore};
use apple_music::client::AppleMusicClient;
use apple_music::updater::AppleArtistQueue;
use auth::{ClientCredentialsManager, SpotifyCredentials, UserTokenManager};
use axum_extra::extract::cookie::Key;

use config::AppConfig;
use db::AppDatabase;
use spotify::client::SpotifyClient;
use state::{AppState, AppStateInner};
use updater::ArtistQueue;

pub async fn build_app() -> Result<Router> {
    // Load .env if present.
    let _ = dotenvy::dotenv().ok();

    // Initialize tracing (no-op on Vercel if you want; keeping it is fine).
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
    let artist_queue = ArtistQueue::new();

    // DB connection
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file or environment");
    let db = AppDatabase::new(&database_url)
        .await
        .expect("Failed to connect to database");
    println!("connected to database at {database_url}");
    migrate!("./migrations")
        .run(&db.pool)
        .await
        .expect("Failed to run database migrations");

    // Load configuration.
    let config = AppConfig::from_env()?;

    let mapbox_key_clone = config.mapbox_private_key.clone();
    let ticketmaster_key_clone = config.ticketmaster_api_key.clone();

    let client = Client::new();

    // Apple Music optional setup
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

    let shared_state = AppState(Arc::new(AppStateInner {
        db,
        client,
        mapbox_key: mapbox_key_clone,
        ticketmaster_key: ticketmaster_key_clone,
        spotify,
        cc_manager,
        user_manager,
        artist_queue,
        spotify_client_id: std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID required"),
        spotify_client_secret: std::env::var("SPOTIFY_CLIENT_SECRET")
            .expect("SPOTIFY_CLIENT_SECRET required"),

        apple_music_client: apple_client,
        apple_dev_token: apple_dev,
        apple_user_token: apple_user,
        apple_artist_queue: apple_queue,
        cookie_domain: config.cookie_domain.clone(),
        cookie_secure: config.cookie_secure,
        cookie_key: Key::from(config.cookie_key.as_bytes()),
    }));

    let app = axum::Router::new()
        .route("/", axum::routing::get(|| async { "ok" }))
        .route("/health", axum::routing::get(health_check))
        .route(
            "/auth/status",
            axum::routing::get(spotify::spotify_handlers::auth_status),
        )
        .route("/cities", axum::routing::get(mapbox_handlers::get_cities))
        .route(
            "/concerts",
            axum::routing::get(ticketmaster_handlers::get_concerts_tm),
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
        .route(
            "/apple/updater/config",
            axum::routing::post(apple_music_updater_handlers::configure_apple_updater),
        )
        .route(
            "/apple/updater/artists",
            axum::routing::put(apple_music_updater_handlers::update_apple_artists),
        )
        .layer(CorsLayer::permissive())
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
