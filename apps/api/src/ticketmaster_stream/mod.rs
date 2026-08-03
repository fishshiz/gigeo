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
//!
//! This file wires those pieces together into two HTTP handlers —
//! `get_concerts_tm_stream` (incremental ndjson for the map, windowed and
//! fully paginated) and `get_events_by_attraction` (a JSON array of one
//! attraction's upcoming shows, fully paginated) — plus `fetch_events_near`,
//! a non-streaming, non-windowed entry point used by
//! `services::playlist_builder` to find nearby artists without paying for
//! the map's full windowed pagination on every playlist build/update
//! (deliberately Ticketmaster-only — see `get_concerts_tm_stream` for why
//! PredictHQ isn't part of this path).
//!
//! `get_concerts_tm_stream` also reconciles PredictHQ against Ticketmaster:
//! Ticketmaster streams progressively exactly as it always has, and once
//! both fetches complete, PredictHQ-sourced enrichment
//! (`rank`/`predicted_attendance` on a matching Ticketmaster event) and
//! genuinely new PredictHQ-only events are emitted as a second wave. See
//! `crate::events::reconcile_predicthq_events` for the matching itself.

mod client;
mod dates;
mod normalize;
mod types;

pub use types::*;

pub(crate) use normalize::normalize_event;

use crate::error::AppError;
use crate::events::types::EventResponse;
use crate::events::{apply_personalization, dedupe_key, reconcile_predicthq_events};
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

    // Kicked off now so it runs concurrently with the Ticketmaster windowed
    // fetch below rather than adding its own round-trip afterward. Absent
    // entirely (no task spawned) when PREDICTHQ_API_KEY isn't configured —
    // this whole feature is best-effort and must never hold up or break
    // Ticketmaster's own results.
    let predicthq_handle = state.predicthq_api_key.clone().map(|predicthq_key| {
        let client = state.client.clone();
        let within = format!("{radius}mi@{},{}", params.latitude, params.longitude);
        let start_gte = params.start.clone();
        let start_lte = params.end.clone();
        tokio::spawn(async move {
            crate::predicthq::search_concerts(
                &client,
                &predicthq_key,
                &within,
                &start_gte,
                &start_lte,
            )
            .await
        })
    });

    let stream: BoxStream<'static, Result<Bytes, AppError>> = async_stream::try_stream! {
        let mut seen = HashSet::<String>::new();
        let mut ticketmaster_events = Vec::<EventResponse>::new();

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
                        ticketmaster_events.push(event);
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

        // Second wave: reconcile PredictHQ against everything Ticketmaster
        // just streamed. A failed/absent fetch degrades to "no PredictHQ
        // data this request" rather than an error — this feature is
        // additive and must never take down event browsing.
        let predicthq_events = match predicthq_handle {
            Some(handle) => match handle.await {
                Ok(Ok(raw_events)) => raw_events
                    .into_iter()
                    .map(crate::predicthq::normalize_predicthq_event)
                    .collect(),
                Ok(Err(err)) => {
                    tracing::warn!(error = %err, "failed to fetch PredictHQ events, skipping");
                    Vec::new()
                }
                Err(join_err) => {
                    tracing::warn!(error = %join_err, "PredictHQ fetch task failed, skipping");
                    Vec::new()
                }
            },
            None => Vec::new(),
        };

        if !predicthq_events.is_empty() {
            let (enrichments, mut new_events) =
                reconcile_predicthq_events(&ticketmaster_events, predicthq_events);

            for event in enrichments {
                let mut line = serde_json::to_vec(&event)
                    .map_err(|e| -> AppError { AppError::TicketmasterApi { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() } })?;
                line.push(b'\n');
                yield Bytes::from(line);
            }

            // Best-effort image/link backfill for the PredictHQ-only cards
            // (a matched event above is already a Ticketmaster clone with
            // its own real photo/ticket URL, so it's excluded from this).
            // Absent entirely when Apple Music isn't configured or its
            // developer token can't be fetched right now — same
            // never-block-the-feed posture as the PredictHQ fetch itself.
            if let (Some(am), Some(dev_token_mgr)) =
                (&state.apple_music_client, &state.apple_dev_token)
            {
                match dev_token_mgr.get_token().await {
                    Ok(token) => {
                        crate::predicthq::backfill_artwork(
                            &mut new_events,
                            am,
                            &token,
                            &state.apple_artwork_cache,
                        )
                        .await;
                    }
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            "failed to fetch Apple Music developer token, skipping PredictHQ image/link backfill"
                        );
                    }
                }
            }

            for mut event in new_events {
                apply_personalization(&mut event, &personalization_matches);
                let mut line = serde_json::to_vec(&event)
                    .map_err(|e| -> AppError { AppError::TicketmasterApi { status: StatusCode::INTERNAL_SERVER_ERROR, message: e.to_string() } })?;
                line.push(b'\n');
                yield Bytes::from(line);
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
