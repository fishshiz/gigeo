//! HTTP handlers for Spotify-related routes, split by concern:
//! - `artist`: artist lookup (client-credentials auth)
//! - `auth`: OAuth login/callback/status
//! - `playlists`: playlist creation, retrieval, edit, and delete
//! - `db`: persistence for sessions, accounts, and playlists

mod artist;
mod auth;
mod db;
mod playlists;

pub use artist::*;
pub use auth::*;
pub use playlists::*;
