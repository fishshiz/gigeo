//! HTTP handlers for Spotify-related routes, split by concern:
//! - `auth`: OAuth login/callback/status
//! - `playlists`: playlist creation, retrieval, edit, and delete
//! - `db`: persistence for sessions, accounts, and playlists
//!
//! The Client Credentials artist-search handler that used to live here
//! (`GET /artists`) was removed — it had no frontend caller, and its
//! search+verify logic now lives in `crate::artists::worker` as the
//! Spotify fallback half of canonical artist enrichment instead.
//!
//! `resolve_account_from_cookie_lenient` is re-exported at `pub(crate)` for
//! endpoints outside this module (e.g. the concerts stream) where a
//! missing/invalid session should silently disable a feature rather than
//! fail the request.

mod auth;
mod db;
mod playlists;

pub use auth::*;
pub(crate) use db::resolve_account_from_cookie_lenient;
pub use playlists::*;
