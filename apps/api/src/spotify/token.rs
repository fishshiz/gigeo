//! Per-account Spotify token lookup, shared by the create-playlist handler
//! and the periodic playlist updater.

use chrono::{Duration, Utc};

use crate::error::AppError;
use crate::state::AppState;

struct SpotifyTokenRow {
    access_token: String,
    refresh_token: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

/// Returns a valid Spotify access token for the given account, refreshing
/// (and persisting the refresh) if the stored token has expired.
///
/// This looks the token up per-account rather than relying on
/// `state.user_manager`'s single-slot in-memory cache, which only ever holds
/// whichever user most recently completed the OAuth flow in this process.
pub(crate) async fn get_valid_spotify_token(
    state: &AppState,
    account_id: uuid::Uuid,
) -> Result<String, AppError> {
    let row = sqlx::query_as!(
        SpotifyTokenRow,
        r#"
        select access_token, refresh_token, expires_at
        from spotify_account
        where account_id = $1
        "#,
        account_id
    )
    .fetch_optional(&state.db.pool)
    .await?
    .ok_or_else(|| AppError::AuthRequired("Spotify not connected for this account.".into()))?;

    if row.expires_at > Utc::now() + Duration::seconds(60) {
        return Ok(row.access_token);
    }

    let refreshed = state.user_manager.refresh_with(&row.refresh_token).await?;
    let new_refresh_token = refreshed.refresh_token.clone().unwrap_or(row.refresh_token);
    let new_expires_at = Utc::now() + Duration::seconds(refreshed.expires_in as i64);

    sqlx::query!(
        r#"
        update spotify_account
        set access_token = $1, refresh_token = $2, expires_at = $3, updated_at = now()
        where account_id = $4
        "#,
        refreshed.access_token,
        new_refresh_token,
        new_expires_at,
        account_id
    )
    .execute(&state.db.pool)
    .await?;

    Ok(refreshed.access_token)
}
