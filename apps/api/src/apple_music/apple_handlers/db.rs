use crate::accounts::db::resolve_session_account;
use crate::error::AppError;
use crate::state::AppState;
use axum::http::StatusCode;
use axum_extra::extract::cookie::SignedCookieJar;

pub(super) const APPLE_MUSIC_COOKIE: &str = "apple_music_oauth_state";

/// Resolves the account tied to the Apple Music session cookie. Kept
/// entirely independent of Spotify's `spotify_oauth_state` cookie/session
/// so a user can connect both providers in the same browser session.
pub(super) async fn resolve_account_from_cookie(
    state: &AppState,
    jar: &SignedCookieJar,
) -> Result<uuid::Uuid, AppError> {
    let cookie = jar.get(APPLE_MUSIC_COOKIE).ok_or_else(|| {
        AppError::AuthRequired("No Apple Music session cookie. Connect Apple Music first.".into())
    })?;
    let session_id = uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
        status: StatusCode::UNAUTHORIZED,
        message: "Invalid session cookie".into(),
    })?;
    resolve_session_account(&state.db.pool, session_id)
        .await?
        .ok_or_else(|| {
            AppError::AuthRequired(
                "Apple Music session expired or invalid. Connect Apple Music again.".into(),
            )
        })
}

/// Upserts the account keyed by `apple_music_user_id` — a client-generated
/// persistent UUID (`localStorage`, see `hooks/appleMusic.ts`), since Apple
/// gives no stable user identifier via MusicKit. The stored Music User
/// Token is simply overwritten on each connect; MusicKit tokens have no
/// refresh flow, so a re-connect from the same device is the only way a
/// stale token gets replaced.
pub(super) async fn upsert_apple_music_account(
    db: &sqlx::PgPool,
    device_id: &str,
    music_user_token: &str,
    storefront: &str,
) -> Result<uuid::Uuid, sqlx::Error> {
    let mut tx = db.begin().await?;

    let existing = sqlx::query!(
        r#"select account_id from apple_music_account where apple_music_user_id = $1"#,
        device_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let account_id = match existing {
        Some(row) => row.account_id,
        None => {
            let acc = sqlx::query!(
                r#"insert into account (provider) values ('apple_music') returning id"#
            )
            .fetch_one(&mut *tx)
            .await?;
            acc.id
        }
    };

    sqlx::query!(
        r#"
        insert into apple_music_account (
            account_id, apple_music_user_id, music_user_token, storefront, token_created_at
        )
        values ($1, $2, $3, $4, now())
        on conflict (account_id) do update set
            music_user_token = excluded.music_user_token,
            storefront = excluded.storefront,
            token_created_at = now(),
            updated_at = now()
        "#,
        account_id,
        device_id,
        music_user_token,
        storefront,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(account_id)
}

/// `pub(crate)` (unlike the rest of this module) because `playlist_updater`
/// needs it directly to resolve a claimed Apple Music playlist's token.
#[derive(sqlx::FromRow, Debug, Clone)]
pub(crate) struct AppleMusicAccountRow {
    pub account_id: uuid::Uuid,
    pub music_user_token: String,
    pub storefront: String,
}

pub(crate) async fn get_apple_music_account(
    db: &sqlx::PgPool,
    account_id: uuid::Uuid,
) -> Result<Option<AppleMusicAccountRow>, sqlx::Error> {
    sqlx::query_as!(
        AppleMusicAccountRow,
        r#"
        select account_id, music_user_token, storefront
        from apple_music_account
        where account_id = $1
        "#,
        account_id
    )
    .fetch_optional(db)
    .await
}
