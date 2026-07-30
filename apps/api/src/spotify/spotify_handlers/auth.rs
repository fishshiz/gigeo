use super::db::{
    create_session, get_spotify_account, resolve_session_account, upsert_spotify_account,
};
use crate::cookie::utils::build_session_cookie;
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// OAuth callback handlers
// ---------------------------------------------------------------------------

/// `GET /login` — redirects the user to Spotify's authorization page.
pub async fn login(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    let state_param = generate_state();
    let url = state.user_manager.authorize_url(&state_param);

    (jar, Redirect::to(&url))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
    pub state: Option<String>,
}

/// `GET /callback` — Spotify redirects here after user authorization.
/// Exchanges the authorization code for access + refresh tokens.
pub async fn oauth_callback(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(
        "callback hit: code={:?}, state={:?}, error={:?}",
        query.code,
        query.state,
        query.error
    );
    if let Some(err) = query.error {
        return Err(AppError::SpotifyApi {
            status: StatusCode::BAD_REQUEST,
            message: format!("Spotify error: {}", err),
        });
    }
    let token = state
        .user_manager
        .exchange_code(query.code.unwrap_or_default().as_str())
        .await?;

    let me = state
        .user_manager
        .get_current_user(&token.access_token)
        .await?;
    let account_id = upsert_spotify_account(&state.db.pool, &token, &me.id).await?;
    let session_id = create_session(&state.db.pool, account_id).await?;

    let jar = jar.add(build_session_cookie(&state, session_id));

    Ok((jar, Redirect::to("/")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| format!("{:02x}", rng.r#gen::<u8>()))
        .collect()
}

#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    logged_in: bool,
    spotify_connected: bool,
    spotify_user_id: Option<String>,
}
pub async fn auth_status(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<AuthStatusResponse>, AppError> {
    let cookie = match jar.get("spotify_oauth_state") {
        Some(c) => c,
        None => {
            return Ok(Json(AuthStatusResponse {
                logged_in: false,
                spotify_connected: false,
                spotify_user_id: None,
            }));
        }
    };

    let session_id = uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
        status: StatusCode::UNAUTHORIZED,
        message: "Unauthorized".into(),
    })?;

    let Some(account_id) = resolve_session_account(&state.db.pool, session_id).await? else {
        return Ok(Json(AuthStatusResponse {
            logged_in: false,
            spotify_connected: false,
            spotify_user_id: None,
        }));
    };

    let spotify = get_spotify_account(&state.db.pool, account_id).await?;

    let resp = match spotify {
        Some(s) => AuthStatusResponse {
            logged_in: true,
            spotify_connected: true,
            spotify_user_id: Some(s.spotify_user_id.clone()),
        },
        None => AuthStatusResponse {
            logged_in: true,
            spotify_connected: false,
            spotify_user_id: None,
        },
    };

    Ok(Json(resp))
}
