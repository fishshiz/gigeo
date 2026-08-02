//! HTTP handlers for Spotify-related routes, split by concern:
//! - `artist`: artist lookup (client-credentials auth)
//! - `auth`: OAuth login/callback/status
//! - `playlists`: playlist creation, retrieval, edit, and delete
//! - `db`: persistence for sessions, accounts, and playlists
//!
//! `resolve_account_from_cookie_lenient` is re-exported at `pub(crate)` for
//! endpoints outside this module (e.g. the concerts stream) where a
//! missing/invalid session should silently disable a feature rather than
//! fail the request.

mod artist;
mod auth;
mod db;
mod playlists;

pub use artist::*;
pub use auth::*;
pub(crate) use db::resolve_account_from_cookie_lenient;
pub use playlists::*;
