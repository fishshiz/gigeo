// src/lib.rs
mod apple_music;
mod apple_music_updater_handlers;
mod auth;
mod config;
mod error;
mod spotify;
mod spotify_handlers;
mod state;
mod ticketmaster_handlers;
mod updater;

use std::sync::Arc;

use anyhow::Result;
use axum::response::IntoResponse;
use axum::{Json, Router};
use reqwest::Client;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

use apple_music::auth::{AppleMusicCredentials, DeveloperTokenManager, MusicUserTokenStore};
use apple_music::client::AppleMusicClient;
use apple_music::updater::AppleArtistQueue;
use auth::{ClientCredentialsManager, SpotifyCredentials, UserTokenManager};
use config::AppConfig;
use spotify::SpotifyClient;
use state::AppState;
use updater::ArtistQueue;

pub fn build_app() -> Result<Router> {
    // Load .env if present.
    let _ = dotenvy::dotenv();

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
        apple_artist_queue: apple_queue,
    });

    let app = axum::Router::new()
        .route("/health", axum::routing::get(health_check))
        .route(
            "/cities",
            axum::routing::get(ticketmaster_handlers::get_cities),
        )
        .route(
            "/concerts",
            axum::routing::get(ticketmaster_handlers::get_concerts_tm),
        )
        .route(
            "/artists",
            axum::routing::get(spotify_handlers::get_artist_info),
        )
        .route(
            "/spotify/playlist",
            axum::routing::post(spotify_handlers::create_playlist),
        )
        .route(
            "/spotify/login",
            axum::routing::get(spotify_handlers::login),
        )
        .route(
            "/spotify/callback",
            axum::routing::get(spotify_handlers::oauth_callback),
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

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}
