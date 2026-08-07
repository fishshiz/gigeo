use crate::state::AppState;
use axum::extract::FromRef;
use axum_extra::extract::cookie::{Cookie, Key, SameSite};

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

pub fn build_session_cookie(
    state: &AppState,
    session_id: uuid::Uuid,
    cookie_name: &'static str,
) -> Cookie<'static> {
    let mut cookie = Cookie::build((cookie_name, session_id.to_string()))
        .http_only(true)
        .secure(state.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::days(30))
        .build();

    if let Some(domain) = &state.cookie_domain {
        cookie.set_domain(domain.clone());
    }

    cookie
}

/// A same-named, same-path/domain cookie with `max_age(0)`, so the
/// `Set-Cookie` it produces clears the original in the browser instead of
/// setting an unrelated one — clearing requires matching attributes, not
/// just the name.
pub fn build_expired_cookie(state: &AppState, cookie_name: &'static str) -> Cookie<'static> {
    let mut cookie = Cookie::build((cookie_name, ""))
        .http_only(true)
        .secure(state.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::ZERO)
        .build();

    if let Some(domain) = &state.cookie_domain {
        cookie.set_domain(domain.clone());
    }

    cookie
}
