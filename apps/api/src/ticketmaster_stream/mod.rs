//! All access to the Ticketmaster Discovery API, split by concern:
//! - `types`: raw Ticketmaster response shapes and `PageLimit` (how far a
//!   caller pages through results) — the normalized `EventResponse` shape
//!   lives in `crate::events::types` instead, since it isn't
//!   Ticketmaster-specific
//! - `dates`: splitting a datetime range into per-calendar-day windows
//! - `client`: fetching a single page from the Ticketmaster API, and the
//!   `should_fetch_next` pagination decision
//! - `normalize`: converting a raw `TmEvent` into an `EventResponse` — the
//!   one place in this module that still needs to know Ticketmaster's raw
//!   shape; `apply_personalization`/`dedupe_key` live in `crate::events`
//!   since they only ever touch the normalized shape
//! - `source`: `TicketmasterSource`, this provider's `crate::events::source::EventSource`
//!   adapter — the seam `crate::events::merge` merges against. The live
//!   `/concerts/stream` handler itself lives in `crate::events::stream`
//!   now, since it's genuinely cross-provider (also depends on
//!   `crate::predicthq`), not Ticketmaster-specific.
//!
//! This file wires those pieces together into `get_events_by_attraction` (a
//! JSON array of one attraction's upcoming shows, fully paginated) plus
//! `fetch_events_near`, a non-streaming, non-windowed entry point used by
//! `services::playlist_builder` to find nearby artists without paying for
//! the map's full windowed pagination on every playlist build/update
//! (deliberately Ticketmaster-only, same reasoning as always — PredictHQ
//! reconciliation is only worth it for the map's live browsing view).

mod client;
mod dates;
mod normalize;
pub(crate) mod source;
mod types;

pub use types::*;

pub(crate) use dates::date_windows;
pub(crate) use normalize::normalize_event;

use crate::error::AppError;
use crate::events::dedupe_key;
use crate::events::types::EventResponse;
use crate::state::AppState;
use axum::{
    Json,
    extract::{Query, State},
};
use client::{fetch_tm_attraction_page, fetch_tm_page, should_fetch_next};
use std::collections::HashSet;

/// `GET /future-events?id=...` — every upcoming event for one attraction,
/// fully paginated (an artist with many announced shows shouldn't be
/// silently truncated).
pub async fn get_events_by_attraction(
    State(state): State<AppState>,
    Query(query): Query<AttractionEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, AppError> {
    let events = fetch_events_for_attraction(
        &state.client,
        &state.ticketmaster_key,
        &query.id,
        PageLimit::All,
    )
    .await?;

    Ok(Json(events))
}

/// Fetches events within `radius` miles of `geo_hash` over `[start, end]`,
/// normalized and deduped. Unlike `crate::events::get_concerts_tm_stream`,
/// this doesn't window by calendar day — `page_limit` alone controls how
/// far it pages into the (single) date range, so `PageLimit::First` costs
/// exactly one Ticketmaster request.
///
/// Used by `services::playlist_builder`, which calls this on a 5-minute
/// timer (the periodic updater) and doesn't need the map stream's full
/// completeness.
pub async fn fetch_events_near(
    client: &reqwest::Client,
    api_key: &str,
    geo_hash: &str,
    radius: u8,
    start: &str,
    end: &str,
    page_limit: PageLimit,
) -> Result<Vec<EventResponse>, AppError> {
    let mut events = Vec::new();
    let mut seen = HashSet::<String>::new();
    let mut page = 0_u32;

    loop {
        let body = fetch_tm_page(client, api_key, geo_hash, radius, start, end, page).await?;

        for raw in body.embedded.map(|e| e.events).unwrap_or_default() {
            let event = normalize_event(raw);
            if seen.insert(dedupe_key(&event)) {
                events.push(event);
            }
        }

        match &body.page {
            Some(meta) if should_fetch_next(page_limit, meta) => page += 1,
            _ => break,
        }
    }

    Ok(events)
}

/// Fetches events for one attraction, normalized and deduped, paging
/// according to `page_limit`. No date/geo filter — Ticketmaster's
/// attraction lookup doesn't take either.
async fn fetch_events_for_attraction(
    client: &reqwest::Client,
    api_key: &str,
    attraction_id: &str,
    page_limit: PageLimit,
) -> Result<Vec<EventResponse>, AppError> {
    let mut events = Vec::new();
    let mut seen = HashSet::<String>::new();
    let mut page = 0_u32;

    loop {
        let body = fetch_tm_attraction_page(client, api_key, attraction_id, page).await?;

        for raw in body.embedded.map(|e| e.events).unwrap_or_default() {
            let event = normalize_event(raw);
            if seen.insert(dedupe_key(&event)) {
                events.push(event);
            }
        }

        match &body.page {
            Some(meta) if should_fetch_next(page_limit, meta) => page += 1,
            _ => break,
        }
    }

    Ok(events)
}
