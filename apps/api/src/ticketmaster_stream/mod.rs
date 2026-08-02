//! Streams nearby events from the Ticketmaster Discovery API as
//! newline-delimited JSON, split by concern:
//! - `types`: raw Ticketmaster response shapes and this service's normalized
//!   response shapes
//! - `dates`: splitting a datetime range into per-calendar-day windows
//! - `client`: fetching a single page from the Ticketmaster API
//! - `normalize`: converting raw events into the normalized shape and
//!   deriving a dedupe key
//!
//! This file wires those pieces together into the streaming HTTP handler.

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
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use axum_extra::extract::cookie::SignedCookieJar;
use bytes::Bytes;
use client::fetch_tm_page;
use dates::date_windows;
use futures::{StreamExt, stream::BoxStream};
use geohash::{Coord, encode};
use std::collections::HashSet;

pub async fn get_concerts_tm_stream(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(params): Query<EventsQuery>,
) -> Result<Response, AppError> {
    // Personalized discovery is best-effort: no session, no Spotify
    // connection, or a failed fetch should silently disable it rather than
    // break event browsing for everyone.
    let top_artist_names = match resolve_account_from_cookie_lenient(&state, &jar).await {
        Some(account_id) => {
            crate::spotify::top_artists::get_top_artist_names(&state, account_id)
                .await
                .unwrap_or_else(|err| {
                    tracing::warn!(
                        error = %err,
                        "failed to fetch top artists for personalized discovery, skipping"
                    );
                    HashSet::new()
                })
        }
        None => HashSet::new(),
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
                    apply_personalization(&mut event, &top_artist_names);
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

                if meta.number + 1 >= meta.totalPages {
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
