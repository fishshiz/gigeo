//! Canonical artist identity and enrichment — see
//! `docs/adr/0001-canonical-artist-model.md` for the design reasoning,
//! including why this has no foreign key to events or performers (neither
//! is persisted anywhere in this system).
//!
//! There's no batch job to trigger enrichment: `spawn_enrichment` is called
//! from the live Ticketmaster/PredictHQ request path
//! (`ticketmaster_stream::get_concerts_tm_stream`) with every performer name
//! seen in that request, and detaches a bounded-concurrency background task
//! that resolves whichever of them are new or stale, without blocking the
//! response already streaming back to that caller. The first request to see
//! a given artist won't have enrichment for it yet; every request after
//! does.
//!
//! Both Apple Music and Spotify are always looked up (verified
//! exact-normalized-name match against each provider's own catalog
//! search), concurrently — but Apple Music wins identity/display fields
//! whenever it has a match (display_name, artwork, genres,
//! similar_artists), mirroring the priority already established by
//! `crate::apple_music::artwork_cache` and `crate::spotify::top_artists`.
//! A Spotify match found alongside an Apple one only ever contributes its
//! own id/listen link on top — see `worker::resolve_fresh`. Similar/
//! related artists only ever come from Apple Music's `similar-artists`
//! view (Spotify deprecated its own for new API access — see
//! `crate::spotify::top_artists` module docs), so an artist that only
//! Spotify could verify simply has no similar-artists list.

mod cleaning;
mod db;
mod lookup;
#[cfg(test)]
mod matching_eval;
#[cfg(test)]
mod spotify_backfill;
mod worker;

pub(crate) use lookup::attach_enrichment;
pub(crate) use worker::spawn_enrichment;

use serde::{Deserialize, Serialize};

/// One performer name seen in a live request, with a Ticketmaster
/// attraction id when the source event carried one. Not every sighting has
/// one — PredictHQ-sourced performers never do (their `id` is a PredictHQ
/// entity id, a different namespace — see `crate::predicthq::normalize`).
pub(crate) struct ArtistCandidate {
    pub name: String,
    pub ticketmaster_attraction_id: Option<String>,
}

/// A denormalized snapshot of one Apple Music similar-artist result,
/// stored as-is (jsonb) rather than as its own canonicalized row — a
/// similar artist is a "nice to have" related link, not something this
/// system needs to resolve identity for in its own right.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SimilarArtist {
    pub name: String,
    pub apple_music_id: String,
    pub apple_music_url: Option<String>,
    pub artwork_url: Option<String>,
}
