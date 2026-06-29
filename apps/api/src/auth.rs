//! Spotify authentication: Client Credentials for public data,
//! Authorization Code flow for user-scoped operations (playlists).
//!
//! Reference:
//!   Client Credentials – https://developer.spotify.com/documentation/web-api/tutorials/client-credentials-flow
//!   Authorization Code – https://developer.spotify.com/documentation/web-api/tutorials/code-flow
//!   Token Refresh      – https://developer.spotify.com/documentation/web-api/tutorials/refreshing-tokens

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Loaded from environment variables at startup.
#[derive(Clone)]
pub struct SpotifyCredentials {
    pub client_id: String,
    pub client_secret: String,
    /// Only needed for the Authorization Code flow callback.
    /// Must be HTTPS (or http://127.0.0.1 for local dev).
    pub redirect_uri: String,
}

impl SpotifyCredentials {
    pub fn from_env() -> Self {
        Self {
            client_id: std::env::var("SPOTIFY_CLIENT_ID").expect("SPOTIFY_CLIENT_ID must be set"),
            client_secret: std::env::var("SPOTIFY_CLIENT_SECRET")
                .expect("SPOTIFY_CLIENT_SECRET must be set"),
            redirect_uri: std::env::var("SPOTIFY_REDIRECT_URI")
                .unwrap_or_else(|_| "http://127.0.0.1:3000/callback".into()),
        }
    }

    fn basic_header(&self) -> String {
        let encoded = B64.encode(format!("{}:{}", self.client_id, self.client_secret));
        format!("Basic {encoded}")
    }
}

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    /// Only present in Authorization Code flow responses.
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// A cached token with its expiry instant.
#[derive(Clone)]
struct CachedToken {
    access_token: String,
    expires_at: std::time::Instant,
}

impl CachedToken {
    fn is_expired(&self) -> bool {
        // Refresh 60 s before actual expiry to avoid edge-case failures.
        std::time::Instant::now() >= self.expires_at - std::time::Duration::from_secs(60)
    }
}

// ---------------------------------------------------------------------------
// Client Credentials token manager
// ---------------------------------------------------------------------------

/// Manages a Client Credentials access token with automatic refresh.
/// Used for public, non-user endpoints (search, artist info).
pub struct ClientCredentialsManager {
    creds: SpotifyCredentials,
    http: Client,
    cached: RwLock<Option<TokenResponse>>,
}

impl ClientCredentialsManager {
    pub fn new(creds: SpotifyCredentials, http: Client) -> Arc<Self> {
        Arc::new(Self {
            creds,
            http,
            cached: RwLock::new(None),
        })
    }

    /// Returns a valid access token, refreshing if necessary.
    pub async fn get_token(&self) -> Result<TokenResponse, AppError> {
        // Fast path: read lock.

        let resp = self
            .http
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", self.creds.basic_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body("grant_type=client_credentials")
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::SpotifyApi {
                status,
                message: body,
            });
        }

        let tok: TokenResponse = resp.json().await?;

        tracing::info!(
            "Client Credentials token refreshed (expires in {}s)",
            tok.expires_in
        );
        Ok(tok)
    }
}

async fn upsert_spotify_account(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    spotify_user_id: &str,
    token: &TokenResponse,
) -> Result<(), sqlx::Error> {
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token.expires_in as i64);

    sqlx::query!(
        r#"
    insert into spotify_account (
        user_id, spotify_user_id, access_token, refresh_token, token_type, expires_at
    )
    values ($1, $2, $3, $4, $5, $6)
    on conflict (user_id) do update set
        access_token = excluded.access_token,
        refresh_token = case
            when excluded.refresh_token = '' then spotify_account.refresh_token
            else excluded.refresh_token
        end,
        token_type = excluded.token_type,
        expires_at = excluded.expires_at,
        updated_at = now()
    "#,
        user_id,
        spotify_user_id,
        token.access_token,
        token.refresh_token.clone().unwrap_or_default(),
        token.token_type,
        expires_at,
    )
    .execute(db)
    .await?;

    Ok(())
}
// ---------------------------------------------------------------------------
// Authorization Code token manager (user-scoped)
// ---------------------------------------------------------------------------

/// Stores a single user's token pair obtained via the Authorization Code flow.
/// In production you would store per-user tokens in a database; this is a
/// single-user demo.
pub struct UserTokenManager {
    creds: SpotifyCredentials,
    http: Client,
    token: RwLock<Option<UserToken>>,
}

#[derive(Deserialize)]
pub struct SpotifyMe {
    pub id: String,
    pub account_id: String,
    pub display_name: Option<String>,
}

#[derive(Clone)]
struct UserToken {
    access_token: String,
    refresh_token: String,
    expires_at: std::time::Instant,
}

impl UserTokenManager {
    pub fn new(creds: SpotifyCredentials, http: Client) -> Arc<Self> {
        Arc::new(Self {
            creds,
            http,
            token: RwLock::new(None),
        })
    }

    /// Build the Spotify authorization URL the user should visit.
    pub fn authorize_url(&self, state: &str) -> String {
        let scopes = "playlist-modify-public playlist-modify-private user-read-private";
        format!(
            "https://accounts.spotify.com/authorize?response_type=code&show_dialog=true\
             &client_id={client_id}\
             &scope={scopes}\
             &redirect_uri={redirect_uri}\
             &state={state}",
            client_id = self.creds.client_id,
            scopes = urlencoding(scopes),
            redirect_uri = urlencoding(&self.creds.redirect_uri),
            state = urlencoding(state),
        )
    }

    /// Exchange the authorization `code` for access + refresh tokens.
    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, AppError> {
        let body = format!(
            "grant_type=authorization_code&code={}&redirect_uri={}",
            urlencoding(code),
            urlencoding(&self.creds.redirect_uri),
        );

        tracing::info!(
            redirect_uri = %self.creds.redirect_uri,
            code_prefix = %code.chars().take(8).collect::<String>(),
            "exchanging spotify auth code"
        );
        let resp = self
            .http
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", self.creds.basic_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::SpotifyApi {
                status,
                message: text,
            });
        }

        let tok: TokenResponse = resp.json().await?;
        let mut guard = self.token.write().await;
        let access_token = &tok.access_token;
        let refresh_token = tok.refresh_token.as_ref().map(|s| s.clone());
        *guard = Some(UserToken {
            access_token: access_token.clone(),
            refresh_token: refresh_token
                .ok_or_else(|| AppError::Internal("No refresh token in response".into()))?,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(tok.expires_in),
        });

        tracing::info!("User token obtained (expires in {}s)", tok.expires_in);
        Ok(tok)
    }

    /// Returns a valid user access token, refreshing automatically if expired.
    pub async fn get_token(&self) -> Result<String, AppError> {
        // Fast path.
        {
            let guard = self.token.read().await;
            match guard.as_ref() {
                Some(t) if !t.is_expired() => return Ok(t.access_token.clone()),
                Some(_) => { /* expired, fall through to refresh */ }
                None => {
                    return Err(AppError::AuthRequired(
                        "User has not authorized. Visit /login first.".into(),
                    ));
                }
            }
        }

        // Refresh.
        let mut guard = self.token.write().await;
        let current = guard.as_ref().ok_or_else(|| {
            AppError::AuthRequired("User has not authorized. Visit /login first.".into())
        })?;

        // Double-check.
        if !current.is_expired() {
            return Ok(current.access_token.clone());
        }

        let body = format!(
            "grant_type=refresh_token&refresh_token={}",
            urlencoding(&current.refresh_token),
        );

        let resp = self
            .http
            .post("https://accounts.spotify.com/api/token")
            .header("Authorization", self.creds.basic_header())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::SpotifyApi {
                status,
                message: text,
            });
        }

        let tok: TokenResponse = resp.json().await?;
        let new_refresh = tok
            .refresh_token
            .unwrap_or_else(|| current.refresh_token.clone());

        *guard = Some(UserToken {
            access_token: tok.access_token.clone(),
            refresh_token: new_refresh,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(tok.expires_in),
        });

        tracing::info!("User token refreshed (expires in {}s)", tok.expires_in);
        Ok(tok.access_token)
    }

    pub async fn is_authenticated(&self) -> bool {
        self.token.read().await.is_some()
    }

    pub async fn get_current_user(&self, access_token: &str) -> Result<SpotifyMe, AppError> {
        let resp = self
            .http
            .get("https://api.spotify.com/v1/me")
            .bearer_auth(access_token)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::SpotifyApi {
                status,
                message: text,
            });
        }

        Ok(resp.json().await?)
    }
}
fn token_is_expired(expires_in: u64) -> bool {
    std::time::Instant::now()
        >= (std::time::Instant::now() + std::time::Duration::from_secs(expires_in))
            - std::time::Duration::from_secs(60)
}

impl UserToken {
    fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at - std::time::Duration::from_secs(60)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Percent-encode a string for use in URL query parameters.
fn urlencoding(s: &str) -> String {
    // Simple encoding sufficient for the parameters we use.
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('+', "%2B")
        .replace('@', "%40")
}
