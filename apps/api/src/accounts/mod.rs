//! Provider-agnostic account/session/playlist persistence, shared by
//! `spotify::spotify_handlers` and `apple_music::apple_handlers`.
//!
//! `account`, `user_session`, and `playlist` are generic tables (see
//! `docs/adr/0001-canonical-artist-model.md`'s sibling migration
//! `001_initial.sql`) — a `user_session` row points at a generic
//! `account_id` regardless of which provider (`spotify_account` /
//! `apple_music_account`) owns it, so this logic works unchanged for
//! either.

pub mod db;
