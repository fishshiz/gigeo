//! HTTP handlers for the Apple Music "connect" and playlist-management
//! routes, split by concern — mirrors `spotify::spotify_handlers`'s
//! layout:
//! - `auth`: MusicKit connect/logout/status (no redirect-based OAuth —
//!   the Music User Token is obtained client-side via MusicKit JS)
//! - `playlists`: playlist creation, retrieval, cadence edit, and
//!   remove-from-gigeo (see `playlists`'s module doc for why edit/delete
//!   are narrower than Spotify's)
//! - `db`: persistence for Apple Music accounts

mod auth;
mod db;
mod playlists;

pub use auth::*;
pub use playlists::*;

/// Re-exported at `pub(crate)` for `services::playlist_updater`, which
/// needs the stored Music User Token directly to run a claimed Apple
/// Music playlist's scheduled update.
pub(crate) use db::get_apple_music_account;
