use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Spotify API error ({status}): {message}")]
    SpotifyApi { status: u16, message: String },

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
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::SpotifyApi { status, message } => {
                let code =
                    StatusCode::from_u16(*status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
                (code, message.clone())
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
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
        };

        let body = json!({ "error": message });
        (status, Json(body)).into_response()
    }
}
