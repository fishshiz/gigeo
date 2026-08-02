use crate::auth::TokenResponse;
use crate::error::AppError;
use crate::services::playlist_builder::{PlaylistUpdateMode, PlaylistVisibility};
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

pub(super) async fn create_session(
    db: &sqlx::PgPool,
    account_id: uuid::Uuid,
) -> Result<uuid::Uuid, sqlx::Error> {
    let session_id = uuid::Uuid::new_v4();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query!(
        r#"
        insert into user_session (id, expires_at, account_id)
        values ($1, $2, $3)
        "#,
        session_id,
        expires_at,
        account_id
    )
    .execute(db)
    .await?;

    Ok(session_id)
}

pub(super) struct CreatePlaylistParams<'a> {
    pub account_id: uuid::Uuid,
    pub provider_playlist_id: &'a str,
    pub name: &'a str,
    pub geohash: &'a str, // must be exactly 6 chars — see check constraint
    pub city: &'a str,
    pub update_cadence_days: i16, // must be 7, 30, or 60 — see check constraint
    pub visibility: PlaylistVisibility,
    pub update_mode: PlaylistUpdateMode,
}

pub(super) async fn create_playlist_record(
    db: &sqlx::PgPool,
    params: CreatePlaylistParams<'_>,
) -> Result<uuid::Uuid, sqlx::Error> {
    let next_update_at =
        chrono::Utc::now() + chrono::Duration::days(params.update_cadence_days as i64);

    let row = sqlx::query!(
        r#"
        insert into playlist (
            account_id, provider_playlist_id, name, geohash, city,
            update_cadence_days, visibility, update_mode, next_update_at
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        returning id
        "#,
        params.account_id,
        params.provider_playlist_id,
        params.name,
        params.geohash,
        params.city,
        params.update_cadence_days,
        params.visibility as _,
        params.update_mode as _,
        next_update_at,
    )
    .fetch_one(db)
    .await?;

    Ok(row.id)
}

pub(super) async fn resolve_session_account(
    db: &sqlx::PgPool,
    session_id: uuid::Uuid,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        select account_id
        from user_session
        where id = $1
          and revoked_at is null
          and expires_at > now()
        "#,
        session_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| r.account_id))
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

pub(super) struct PlaylistRow {
    pub provider_playlist_id: String,
    pub is_active: bool,
}

/// Fetches a playlist scoped to the owning account, excluding soft-deleted
/// rows. `None` covers both "wrong account" and "already deleted" — both
/// should look like "not found" to the caller.
pub(super) async fn get_playlist_for_account(
    db: &sqlx::PgPool,
    playlist_id: uuid::Uuid,
    account_id: uuid::Uuid,
) -> Result<Option<PlaylistRow>, sqlx::Error> {
    sqlx::query_as!(
        PlaylistRow,
        r#"
        select provider_playlist_id, is_active
        from playlist
        where id = $1 and account_id = $2 and deleted_at is null
        "#,
        playlist_id,
        account_id
    )
    .fetch_optional(db)
    .await
}

pub(super) struct UpdatePlaylistParams<'a> {
    pub name: &'a str,
    pub visibility: PlaylistVisibility,
    pub update_mode: PlaylistUpdateMode,
    pub update_cadence_days: i16,
}

/// Persists edited config fields only. Deliberately does not touch
/// `next_update_at` — cadence/mode changes take effect on the playlist's
/// already-scheduled next run, not immediately.
pub(super) async fn update_playlist_config(
    db: &sqlx::PgPool,
    playlist_id: uuid::Uuid,
    params: UpdatePlaylistParams<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        update playlist
        set name = $2, visibility = $3, update_mode = $4, update_cadence_days = $5
        where id = $1
        "#,
        playlist_id,
        params.name,
        params.visibility as _,
        params.update_mode as _,
        params.update_cadence_days,
    )
    .execute(db)
    .await
    .map(|_| ())
}

/// Marks a playlist inactive after Spotify confirms it's gone (404).
/// Used by the list-fetch and edit/delete 404 paths — the background
/// updater's own `finalize_playlist_failure` handles this separately
/// since it also manages retry scheduling.
pub(super) async fn deactivate_playlist(
    db: &sqlx::PgPool,
    playlist_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update playlist set is_active = false where id = $1",
        playlist_id
    )
    .execute(db)
    .await
    .map(|_| ())
}

/// Soft-delete: preserves the row (and its playlist_update_run history)
/// while excluding it from all future list/claim queries.
pub(super) async fn soft_delete_playlist(
    db: &sqlx::PgPool,
    playlist_id: uuid::Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update playlist set deleted_at = now() where id = $1",
        playlist_id
    )
    .execute(db)
    .await
    .map(|_| ())
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
