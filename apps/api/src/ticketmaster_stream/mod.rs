//! All access to the Ticketmaster Discovery API, split by concern:
//! - `types`: raw Ticketmaster response shapes, this service's normalized
//!   response shapes, and `PageLimit` (how far a caller pages through
//!   results)
//! - `dates`: splitting a datetime range into per-calendar-day windows
//! - `client`: fetching a single page from the Ticketmaster API, and the
//!   `should_fetch_next` pagination decision
//! - `normalize`: converting raw events into the normalized shape and
//!   deriving a dedupe key
//!
//! This file wires those pieces together into two HTTP handlers —
//! `get_concerts_tm_stream` (incremental ndjson for the map, windowed and
//! fully paginated) and `get_events_by_attraction` (a JSON array of one
//! attraction's upcoming shows, fully paginated) — plus `fetch_events_near`,
//! a non-streaming, non-windowed entry point used by
//! `services::playlist_builder` to find nearby artists without paying for
//! the map's full windowed pagination on every playlist build/update.

mod client;
mod dates;
mod normalize;
mod types;

pub use types::*;

pub(crate) use normalize::{apply_personalization, dedupe_key, normalize_event};

use crate::error::AppError;
use crate::spotify::spotify_handlers::resolve_account_from_cookie_lenient;
use crate::state::AppState;
use axum::{
    Json,
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use axum_extra::extract::cookie::SignedCookieJar;
use bytes::Bytes;
use client::{fetch_tm_attraction_page, fetch_tm_page, should_fetch_next};
use dates::date_windows;
use futures::{StreamExt, stream::BoxStream};
use geohash::{Coord, encode};
use std::collections::{HashMap, HashSet};

pub async fn get_concerts_tm_stream(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(params): Query<EventsQuery>,
) -> Result<Response, AppError> {
    // Personalized discovery is best-effort: no session, no Spotify
    // connection, or a failed fetch should silently disable it rather than
    // break event browsing for everyone.
    let personalization_matches = match resolve_account_from_cookie_lenient(&state, &jar).await {
        Some(account_id) => {
            crate::spotify::top_artists::get_personalization_matches(&state, account_id)
                .await
                .unwrap_or_else(|err| {
                    tracing::warn!(
                        error = %err,
                        "failed to fetch personalized discovery matches, skipping"
                    );
                    HashMap::new()
                })
        }
        None => HashMap::new(),
    };

    let geo_hash = encode(
        Coord {
            x: params.longitude,
            y: params.latitude,
        },
        6,
    )
    .map_err(|e| AppError::TicketmasterApi {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("failed to encode geohash: {e}"),
    })?;

    let windows =
        date_windows(&params.start, &params.end).map_err(|e| AppError::TicketmasterApi {
            status: StatusCode::BAD_REQUEST,
            message: e,
        })?;

    let client = state.client.clone();
    let api_key = state.ticketmaster_key.clone();
    let radius = params.radius;

    let stream: BoxStream<'static, Result<Bytes, AppError>> = async_stream::try_stream! {
        let mut seen = HashSet::<String>::new();

        for (start, end) in windows {
            let mut page = 0_u32;

            loop {
                let body = fetch_tm_page(
                    &client,
                    &api_key,
                    &geo_hash,
                    radius,
                    &start,
                    &end,
                    page,
                ).await?;

                if let Some(meta) = &body.page
                    && meta.totalElements >= 1000
                {
                    tracing::warn!(
                        start = %start,
                        end = %end,
                        page = meta.number,
                        page_size = meta.size,
                        total_pages = meta.totalPages,
                        total_elements = meta.totalElements,
                        "ticketmaster window may exceed deep paging limit"
                    );
                }

                let events = body
                    .embedded
                    .map(|embedded| embedded.events)
                    .unwrap_or_default();

                for raw in events {
                    let mut event = normalize_event(raw);
                    apply_personalization(&mut event, &personalization_matches);
                    let key = dedupe_key(&event);

                    if seen.insert(key) {
                        let mut line = serde_json::to_vec(&event)
                            .map_err(|e| -> AppError { AppError::TicketmasterApi { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() } })?;
                        line.push(b'\n');
                        yield Bytes::from(line);
                    }
                }

                let Some(meta) = body.page else {
                    break;
                };

                if !should_fetch_next(PageLimit::All, &meta) {
                    break;
                }

                page += 1;
            }
        }
    }
    .boxed();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(stream))
        .unwrap())
}

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
/// normalized and deduped. Unlike `get_concerts_tm_stream`, this doesn't
/// window by calendar day — `page_limit` alone controls how far it pages
/// into the (single) date range, so `PageLimit::First` costs exactly one
/// Ticketmaster request.
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
