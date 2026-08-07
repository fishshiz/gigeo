use crate::accounts::db::resolve_session_account;
use crate::auth::TokenResponse;
use crate::error::AppError;
use crate::state::AppState;
use axum::http::StatusCode;
use axum_extra::extract::cookie::SignedCookieJar;

/// Resolves the account tied to the session cookie. Shared by every
/// handler that requires an authenticated user.
pub(super) async fn resolve_account_from_cookie(
    state: &AppState,
    jar: &SignedCookieJar,
) -> Result<uuid::Uuid, AppError> {
    let cookie = jar
        .get("spotify_oauth_state")
        .ok_or_else(|| AppError::AuthRequired("No session cookie. Visit /login first.".into()))?;
    let session_id = uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
        status: StatusCode::UNAUTHORIZED,
        message: "Invalid session cookie".into(),
    })?;
    resolve_session_account(&state.db.pool, session_id)
        .await?
        .ok_or_else(|| {
            AppError::AuthRequired("Session expired or invalid. Visit /login first.".into())
        })
}

/// Lenient variant of `resolve_account_from_cookie`, for endpoints where a
/// missing or invalid session should silently disable a feature (e.g.
/// personalized discovery) rather than fail the whole request.
pub(crate) async fn resolve_account_from_cookie_lenient(
    state: &AppState,
    jar: &SignedCookieJar,
) -> Option<uuid::Uuid> {
    resolve_account_from_cookie(state, jar).await.ok()
}

pub(super) async fn upsert_spotify_account(
    db: &sqlx::PgPool,
    token: &TokenResponse,
    spotify_user_id: &str,
) -> Result<uuid::Uuid, sqlx::Error> {
    let scopes: Vec<String> = token
        .scope
        .clone()
        .unwrap_or_default()
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();

    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token.expires_in as i64);

    let mut tx = db.begin().await?;

    let existing = sqlx::query!(
        r#"select account_id from spotify_account where spotify_user_id = $1"#,
        spotify_user_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let account_id = match existing {
        Some(row) => row.account_id,
        None => {
            let acc =
                sqlx::query!(r#"insert into account (provider) values ('spotify') returning id"#)
                    .fetch_one(&mut *tx)
                    .await?;
            acc.id
        }
    };

    sqlx::query!(
        r#"
        insert into spotify_account (
            account_id, spotify_user_id, access_token, refresh_token,
            token_type, expires_at, scope
        )
        values ($1, $2, $3, $4, $5, $6, $7)
        on conflict (account_id) do update set
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_type = excluded.token_type,
            expires_at = excluded.expires_at,
            scope = excluded.scope,
            updated_at = now()
        "#,
        account_id,
        spotify_user_id,
        token.access_token,
        token.refresh_token.clone().unwrap_or_default(),
        token.token_type,
        expires_at,
        scopes.join(" "),
    )
    .execute(&mut *tx)
    .await?;

    // A fresh OAuth grant may carry different scopes than whatever produced
    // the cached top-artist result (e.g. the account reconnecting after
    // `user-top-read` was added) — drop it so personalization picks up the
    // new token immediately instead of waiting out the cache TTL.
    sqlx::query!(
        "delete from spotify_top_artist_cache where account_id = $1",
        account_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(account_id)
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub(super) struct SpotifyAccountRow {
    pub account_id: uuid::Uuid,
    pub spotify_user_id: String,
    pub access_token: String,
    pub refresh_token: String,
    pub scope: String,
    pub token_type: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub(super) async fn get_spotify_account(
    db: &sqlx::PgPool,
    account_id: uuid::Uuid,
) -> Result<Option<SpotifyAccountRow>, sqlx::Error> {
    sqlx::query_as!(
        SpotifyAccountRow,
        r#"
        select
            account_id,
            spotify_user_id,
            access_token,
            refresh_token,
            scope,
            token_type,
            expires_at
        from spotify_account
        where account_id = $1
        "#,
        account_id
    )
    .fetch_optional(db)
    .await
}
