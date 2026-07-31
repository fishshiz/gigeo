use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Spotify API error ({status}): {message}")]
    SpotifyApi { status: StatusCode, message: String },

    #[error("Ticketmaster API error ({status}): {message}")]
    TicketmasterApi { status: StatusCode, message: String },

    #[error("Unauthorized API error ({status}): {message}")]
    Unauthorized { status: StatusCode, message: String },

    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Artist not found: {0}")]
    ArtistNotFound(String),

    #[error("Authentication required: {0}")]
    AuthRequired(String),

    #[error("Rate limited, retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Invalid query string: {0}")]
    QueryRejection(#[from] axum_extra::extract::QueryRejection),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Playlist not found")]
    PlaylistNotFound,

    #[error("Playlist no longer exists on Spotify")]
    PlaylistUnavailable { playlist_id: uuid::Uuid },
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let AppError::PlaylistUnavailable { playlist_id } = &self {
            return (
                StatusCode::GONE,
                Json(json!({
                    "error": "playlist_unavailable",
                    "playlist_id": playlist_id.to_string(),
                })),
            )
                .into_response();
        }

        let (status, message) = match &self {
            AppError::SpotifyApi { status, message } => {
                let code = *status;
                (code, message.clone())
            }
            AppError::TicketmasterApi { status, message } => {
                let code = *status;
                (code, message.clone())
            }
            AppError::Unauthorized { status, message } => {
                (StatusCode::UNAUTHORIZED, message.clone())
            }
            AppError::Request(e) => (
                StatusCode::BAD_GATEWAY,
                format!("Upstream request failed: {e}"),
            ),
            AppError::ArtistNotFound(name) => {
                (StatusCode::NOT_FOUND, format!("Artist not found: {name}"))
            }
            AppError::AuthRequired(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::RateLimited { retry_after_secs } => (
                StatusCode::TOO_MANY_REQUESTS,
                format!("Rate limited. Retry after {retry_after_secs}s"),
            ),
            AppError::QueryRejection(e) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid query string: {e}"),
            ),
            AppError::Database(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {e}"),
            ),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::PlaylistNotFound => {
                (StatusCode::NOT_FOUND, "Playlist not found".to_string())
            }
            AppError::PlaylistUnavailable { .. } => unreachable!("handled above"),
        };

        let body = json!({ "error": message });
        (status, Json(body)).into_response()
    }
}
