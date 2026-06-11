use crate::state::AppState;
use axum::extract::FromRef;
use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}

pub fn build_session_cookie(state: &AppState, session_id: uuid::Uuid) -> Cookie<'static> {
    let mut cookie = Cookie::build(("session_id", session_id.to_string()))
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

fn build_expired_session_cookie(state: &AppState) -> Cookie<'static> {
    let mut cookie = Cookie::build(("session_id", ""))
        .http_only(true)
        .secure(state.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build();

    if let Some(domain) = &state.cookie_domain {
        cookie.set_domain(domain.clone());
    }

    cookie
}
