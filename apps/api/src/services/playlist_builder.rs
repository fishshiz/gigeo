
use crate::state::{AppState};
use crate::error::{AppError};
use crate::ticketmaster_stream::{
    EventsQuery, EventResponse, TicketmasterResponse, dedupe_key, normalize_event,
};
use geohash::{encode, Coord};
use std::collections::HashSet;
use tracing::info;

pub async fn get_concerts_tm_impl(
    state: &AppState,
    params: &EventsQuery,
) -> Result<Vec<EventResponse>, AppError> {
    let hash = encode(
        Coord {
            x: params.longitude,
            y: params.latitude,
        },
        6,
    )
    .map_err(|e| AppError::Internal(format!("Failed to encode geohash: {e}")))?;

    let url = format!(
        "https://app.ticketmaster.com/discovery/v2/events.json?geoPoint={}&apikey={}&radius={}&startDateTime={}&endDateTime={}&size=200&sort=date,asc",
        hash,
        state.ticketmaster_key,
        params.radius,
        params.start,
        params.end
    );
    info!(
        "ticketmaster request: geoPoint={} radius={} start={} end={}",
        hash, params.radius, params.start, params.end
    );
    let resp = state.client.get(&url).send().await.map_err(AppError::Request)?;

    let status = resp.status();
    let text = resp.text().await.map_err(AppError::Request)?;
    info!("status, text, {}, {}", status, text);

    if !status.is_success() {
        return Err(AppError::TicketmasterApi {
            status,
            message: text,
        });
    }

    let body: TicketmasterResponse = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("Ticketmaster decode error: {e}")))?;

    let mut seen = HashSet::<String>::new();
    let events: Vec<EventResponse> = body
        .embedded
        .into_iter()
        .flat_map(|e| e.events)
        .map(normalize_event)
        .filter(|event| seen.insert(dedupe_key(event)))
        .collect();

    Ok(events)
}


