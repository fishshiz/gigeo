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

pub(crate) use lookup::attach_genres;
pub(crate) use worker::spawn_enrichment;

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::events::types::ArtistEnrichment;
use crate::spotify::client::normalize_artist_name;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct PerformerEnrichmentQuery {
    pub name: String,
    #[serde(rename = "ticketmasterAttractionId")]
    pub ticketmaster_attraction_id: Option<String>,
}

/// `GET /artists/enrichment?name=...&ticketmasterAttractionId=...` —
/// resolves and returns one performer's full canonical-artist enrichment
/// right now, awaiting the same Apple Music/Spotify match `spawn_enrichment`
/// runs in the background for the live list/map stream (a cheap read for
/// an already-matched artist; a real provider lookup, same as any other
/// first sighting, for a brand-new one). Also backfills similar artists if
/// they're still missing — see `worker::backfill_similar_artists` for why
/// that's not already there for an artist matched via the background path.
///
/// Meant to be called once per performer on a selected event's detail view
/// (a handful of calls), not batched across a whole search's worth of
/// performers — see `apps/web/src/EventDetails.tsx`. `None` covers both "no
/// confident match" and any DB/provider error along the way — best-effort,
/// same as everywhere else in this module.
pub async fn get_performer_enrichment(
    State(state): State<AppState>,
    Query(query): Query<PerformerEnrichmentQuery>,
) -> Json<Option<ArtistEnrichment>> {
    let normalized = normalize_artist_name(&query.name);
    if normalized.is_empty() {
        return Json(None);
    }

    worker::resolve_one(
        &state,
        ArtistCandidate {
            name: query.name,
            ticketmaster_attraction_id: query.ticketmaster_attraction_id,
        },
    )
    .await;

    worker::backfill_release_info(&state, &normalized).await;

    let enrichment = lookup::fetch_one(&state.db.pool, &normalized).await;

    let needs_similar_artists = enrichment
        .as_ref()
        .is_some_and(|e| e.similar_artists.is_empty());
    if needs_similar_artists {
        worker::backfill_similar_artists(&state, &normalized).await;
        return Json(lookup::fetch_one(&state.db.pool, &normalized).await);
    }

    Json(enrichment)
}

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
