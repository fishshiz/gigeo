//! `GET /concerts/stream` — the map's incremental ndjson feed. Drives
//! `merge_events` (see `super::merge`) and applies the IO-bound steps
//! (canonical-artist DB enrichment, personalization, Apple Music artwork
//! backfill) that were always request-scoped, never part of the pure
//! merge/ordering logic itself.

use std::collections::HashMap;

use axum::{
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use axum_extra::extract::cookie::SignedCookieJar;
use bytes::Bytes;
use futures::StreamExt;
use geohash::{Coord, encode};

use super::merge::{MergeStep, merge_events};
use super::source::Window;
use super::types::EventResponse;
use super::{apply_personalization, is_music_classified};
use crate::artists::ArtistCandidate;
use crate::error::AppError;
use crate::predicthq::source::PredictHqSource;
use crate::spotify::spotify_handlers::resolve_account_from_cookie_lenient;
use crate::state::AppState;
use crate::ticketmaster_stream::source::TicketmasterSource;
use crate::ticketmaster_stream::{EventsQuery, date_windows};

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

    let raw_windows =
        date_windows(&params.start, &params.end).map_err(|e| AppError::TicketmasterApi {
            status: StatusCode::BAD_REQUEST,
            message: e,
        })?;
    let windows: Vec<Window> = raw_windows
        .into_iter()
        .map(|(start, end)| {
            let calendar_day = chrono::DateTime::parse_from_rfc3339(&start)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            Window {
                start,
                end,
                calendar_day,
            }
        })
        .collect();

    let tm_source = TicketmasterSource::new(
        state.client.clone(),
        state.ticketmaster_key.clone(),
        geo_hash,
        params.radius,
    );

    // Kicked off now so it runs concurrently with Ticketmaster's windowed
    // fetch below rather than adding its own round-trip afterward.
    let within = format!(
        "{}mi@{},{}",
        params.radius, params.latitude, params.longitude
    );
    let phq_source = PredictHqSource::spawn(
        state.client.clone(),
        state.predicthq_api_key.clone(),
        within,
        params.start.clone(),
        params.end.clone(),
    );

    let stream: futures::stream::BoxStream<'static, Result<Bytes, AppError>> = async_stream::try_stream! {
        let mut artist_candidates: Vec<ArtistCandidate> = Vec::new();
        let merged = merge_events(windows, tm_source, phq_source);
        futures::pin_mut!(merged);

        while let Some(step) = merged.next().await {
            match step? {
                MergeStep::Ticketmaster(mut events) => {
                    // Plain indexed reads, not a call to Apple Music/Spotify --
                    // safe to run inline rather than detached like
                    // `spawn_enrichment` below (see `crate::artists::lookup`).
                    crate::artists::attach_enrichment(&mut events, &state.db.pool).await;
                    artist_candidates.extend(candidates_from(&events));

                    for mut event in events {
                        apply_personalization(&mut event, &personalization_matches);
                        yield to_ndjson_line(&event)?;
                    }
                }
                MergeStep::PredictHqEnrichments(mut events) => {
                    // These are clones of the *pre-enrichment* Ticketmaster
                    // events merge_events retained internally for
                    // reconciliation (see merge.rs's `pending`) -- captured
                    // before the MergeStep::Ticketmaster branch above ever
                    // got to mutate its own copy with DB enrichment/
                    // personalization. Confirmed live: without re-running
                    // both here, a cross-source-matched event's re-emission
                    // silently loses performer.enrichment (and any
                    // matched_artist tag), even though it carries the same
                    // event id as an already-enriched first-wave emission.
                    crate::artists::attach_enrichment(&mut events, &state.db.pool).await;

                    for mut event in events {
                        apply_personalization(&mut event, &personalization_matches);
                        yield to_ndjson_line(&event)?;
                    }
                }
                MergeStep::PredictHqNew(mut events) => {
                    // Best-effort image/link backfill for the PredictHQ-only
                    // cards (a matched event is already a Ticketmaster clone
                    // with its own real photo/ticket URL, so it's excluded
                    // from this via MergeStep::PredictHqEnrichments above).
                    // Scoped to this window's events -- a real per-window
                    // batch, not one global batch over the whole request,
                    // so it can't dilute the ordering fix. Absent entirely
                    // when Apple Music isn't configured or its developer
                    // token can't be fetched right now -- same
                    // never-block-the-feed posture as the PredictHQ fetch
                    // itself.
                    if let (Some(am), Some(dev_token_mgr)) =
                        (&state.apple_music_client, &state.apple_dev_token)
                    {
                        match dev_token_mgr.get_token().await {
                            Ok(token) => {
                                crate::predicthq::backfill_artwork(
                                    &mut events,
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

                    // PredictHQ performer ids are PredictHQ entity ids, not
                    // Ticketmaster attraction ids -- a different namespace,
                    // so `ticketmaster_attraction_id` is always `None` here
                    // (see `crate::predicthq::normalize`).
                    // `performer_search_names` falls back to the event's
                    // own title for the ~28% of PredictHQ events with no
                    // structured performer entity at all, same as the
                    // image backfill above.
                    artist_candidates.extend(
                        events
                            .iter()
                            .filter(|event| is_music_classified(event))
                            .flat_map(|event| {
                                crate::predicthq::performer_search_names(event)
                                    .into_iter()
                                    .map(|name| ArtistCandidate {
                                        name,
                                        ticketmaster_attraction_id: None,
                                    })
                            }),
                    );

                    crate::artists::attach_enrichment(&mut events, &state.db.pool).await;

                    for mut event in events {
                        apply_personalization(&mut event, &personalization_matches);
                        yield to_ndjson_line(&event)?;
                    }
                }
            }
        }

        crate::artists::spawn_enrichment(&state, artist_candidates);
    }
    .boxed();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-ndjson")
        .header(header::CACHE_CONTROL, "no-store")
        .body(Body::from_stream(stream))
        .unwrap())
}

/// Every performer name on a Music-classified event, ready to feed
/// canonical-artist enrichment (see `crate::artists::spawn_enrichment`).
fn candidates_from(events: &[EventResponse]) -> Vec<ArtistCandidate> {
    events
        .iter()
        .filter(|event| is_music_classified(event))
        .flat_map(|event| event.performers.iter().flatten())
        .filter_map(|performer| {
            performer.name.clone().map(|name| ArtistCandidate {
                name,
                ticketmaster_attraction_id: performer.id.clone(),
            })
        })
        .collect()
}

fn to_ndjson_line(event: &EventResponse) -> Result<Bytes, AppError> {
    let mut line = serde_json::to_vec(event).map_err(|e| AppError::TicketmasterApi {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: e.to_string(),
    })?;
    line.push(b'\n');
    Ok(Bytes::from(line))
}
