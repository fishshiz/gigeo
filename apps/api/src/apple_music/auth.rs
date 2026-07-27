//! Apple Music authentication: Developer Token (JWT) for catalog data,
//! Music User Token for user-scoped operations (library playlists).
//!
//! References:
//!   Developer Tokens – https://developer.apple.com/documentation/applemusicapi/generating-developer-tokens
//!   User Auth        – https://developer.apple.com/documentation/applemusicapi/user-authentication-for-musickit

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::token_cache::TokenCache;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Loaded from environment variables at startup.
#[derive(Clone)]
pub struct AppleMusicCredentials {
    /// 10-character Team ID from your Apple Developer account.
    pub team_id: String,
    /// 10-character Key ID for your MusicKit private key.
    pub key_id: String,
    /// The PEM-encoded ES256 private key contents.
    pub private_key_pem: String,
}

impl AppleMusicCredentials {
    pub fn from_env() -> Self {
        Self {
            team_id: std::env::var("APPLE_MUSIC_TEAM_ID").expect("APPLE_MUSIC_TEAM_ID must be set"),
            key_id: std::env::var("APPLE_MUSIC_KEY_ID").expect("APPLE_MUSIC_KEY_ID must be set"),
            private_key_pem: std::env::var("APPLE_MUSIC_PRIVATE_KEY")
                .expect("APPLE_MUSIC_PRIVATE_KEY must be set (PEM string)"),
        }
    }
}

// ---------------------------------------------------------------------------
// JWT claims
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct DeveloperTokenClaims {
    /// Issuer: 10-character Team ID.
    iss: String,
    /// Issued-at: seconds since epoch.
    iat: u64,
    /// Expiration: max 6 months (15777000s) from iat.
    exp: u64,
}

// ---------------------------------------------------------------------------
// Developer Token manager
// ---------------------------------------------------------------------------

/// Manages a Developer Token (JWT signed with ES256) for Apple Music API
/// catalog endpoints. The token is cached and regenerated before expiry.
pub struct DeveloperTokenManager {
    creds: AppleMusicCredentials,
    /// We cache the token for most of its lifetime.
    /// Apple allows up to 6 months; we use 12 hours to keep rotation tight.
    token_lifetime_secs: u64,
    cached: TokenCache<String>,
}

impl DeveloperTokenManager {
    pub fn new(creds: AppleMusicCredentials) -> Arc<Self> {
        Arc::new(Self {
            creds,
            token_lifetime_secs: 12 * 60 * 60, // 12 hours
            // Refresh 5 minutes before actual expiry.
            cached: TokenCache::new(std::time::Duration::from_secs(300)),
        })
    }

    /// Returns a valid developer token, regenerating if expired.
    pub async fn get_token(&self) -> Result<String, AppError> {
        if let Some(token) = self.cached.get().await {
            return Ok(token);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| AppError::Internal(format!("System time error: {e}")))?
            .as_secs();

        let claims = DeveloperTokenClaims {
            iss: self.creds.team_id.clone(),
            iat: now,
            exp: now + self.token_lifetime_secs,
        };

        // Build JWT header with kid and ES256 algorithm.
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.creds.key_id.clone());

        let key = EncodingKey::from_ec_pem(self.creds.private_key_pem.as_bytes())
            .map_err(|e| AppError::Internal(format!("Invalid Apple Music private key: {e}")))?;

        let token = encode(&header, &claims, &key)
            .map_err(|e| AppError::Internal(format!("JWT encoding failed: {e}")))?;

        self.cached
            .set(
                token.clone(),
                std::time::Duration::from_secs(self.token_lifetime_secs),
            )
            .await;

        tracing::info!(
            "Apple Music developer token generated (expires in {}s)",
            self.token_lifetime_secs
        );
        Ok(token)
    }
}

// ---------------------------------------------------------------------------
// Music User Token store
// ---------------------------------------------------------------------------

/// Stores a Music User Token provided by the client.
///
/// In a real application, the user obtains this token via MusicKit JS on the
/// frontend and sends it to the backend. The backend stores it here for
/// use with user-scoped endpoints (`/v1/me/...`).
///
/// Music User Tokens do not have a standard refresh flow like OAuth; the
/// frontend must re-authenticate with MusicKit when the token expires.
pub struct MusicUserTokenStore {
    token: RwLock<Option<String>>,
}

impl MusicUserTokenStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            token: RwLock::new(None),
        })
    }

    /// Store a Music User Token (provided by the frontend after MusicKit auth).
    pub async fn set_token(&self, token: String) {
        *self.token.write().await = Some(token);
    }

    /// Retrieve the stored Music User Token.
    pub async fn get_token(&self) -> Result<String, AppError> {
        self.token.read().await.clone().ok_or_else(|| {
            AppError::AuthRequired(
                "Music User Token not set. POST to /apple/user-token first.".into(),
            )
        })
    }
}
