use crate::services::playlist_builder::{PlaylistUpdateMode, PlaylistVisibility};

pub async fn create_session(
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

pub async fn revoke_session(db: &sqlx::PgPool, session_id: uuid::Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update user_session set revoked_at = now() where id = $1",
        session_id
    )
    .execute(db)
    .await
    .map(|_| ())
}

pub async fn resolve_session_account(
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

pub struct CreatePlaylistParams<'a> {
    pub account_id: uuid::Uuid,
    pub provider_playlist_id: &'a str,
    pub name: &'a str,
    pub geohash: &'a str, // must be exactly 6 chars — see check constraint
    pub city: &'a str,
    pub update_cadence_days: i16, // must be 7, 30, or 60 — see check constraint
    pub visibility: PlaylistVisibility,
    pub update_mode: PlaylistUpdateMode,
}

pub async fn create_playlist_record(
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

pub struct PlaylistRow {
    pub provider_playlist_id: String,
    pub is_active: bool,
}

/// Fetches a playlist scoped to the owning account, excluding soft-deleted
/// rows. `None` covers both "wrong account" and "already deleted" — both
/// should look like "not found" to the caller.
pub async fn get_playlist_for_account(
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

pub struct UpdatePlaylistParams<'a> {
    pub name: &'a str,
    pub visibility: PlaylistVisibility,
    pub update_mode: PlaylistUpdateMode,
    pub update_cadence_days: i16,
}

/// Persists edited config fields only. Deliberately does not touch
/// `next_update_at` — cadence/mode changes take effect on the playlist's
/// already-scheduled next run, not immediately.
pub async fn update_playlist_config(
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

/// Narrower sibling of `update_playlist_config`, for providers (Apple
/// Music) whose API doesn't support renaming or changing visibility —
/// cadence is the only field their edit form exposes.
pub async fn update_playlist_cadence(
    db: &sqlx::PgPool,
    playlist_id: uuid::Uuid,
    cadence_days: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update playlist set update_cadence_days = $2 where id = $1",
        playlist_id,
        cadence_days,
    )
    .execute(db)
    .await
    .map(|_| ())
}

/// Marks a playlist inactive after the provider confirms it's gone (404 on
/// Spotify) or after an unrecoverable auth failure (Apple Music — no
/// refresh flow, so an expired/revoked Music User Token surfaces the same
/// way). Used by the list-fetch and edit/delete "gone" paths — the
/// background updater's own `finalize_playlist_failure` handles this
/// separately since it also manages retry scheduling.
pub async fn deactivate_playlist(
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
pub async fn soft_delete_playlist(
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
