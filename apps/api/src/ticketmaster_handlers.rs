use axum::extract::{Query, State};
use axum::{Json, http::StatusCode};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct AttractionEventsQuery {
    id: String,
}

#[derive(Debug, Deserialize)]
struct TicketmasterResponse {
    #[serde(rename = "_embedded")]
    embedded: Option<Embedded>,
}

#[derive(Debug, Deserialize)]
struct Embedded {
    #[serde(default)]
    events: Vec<TmEvent>,
}

#[derive(Debug, Deserialize)]
struct TmEvent {
    id: String,
    url: Option<String>,
    priceRanges: Option<Vec<TmPriceRange>>,
    name: String,
    #[serde(default)]
    images: Vec<Images>,
    dates: TmDate,
    classifications: Option<Vec<TmClassification>>,
    #[serde(rename = "_embedded")]
    embedded: Option<EventEmbedded>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TmPriceRange {
    currency: String,
    min: f32,
    max: f32,
}

#[derive(Debug, Deserialize)]
struct TmDate {
    start: TmDateStart,
}

#[derive(Debug, Deserialize)]
struct TmDateStart {
    // Ticketmaster may omit this for some events
    #[serde(default)]
    dateTime: Option<String>,
    #[serde(default)]
    localDate: Option<String>,
    #[serde(default)]
    localTime: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EventEmbedded {
    venues: Option<Vec<TmVenue>>,
    attractions: Option<Vec<TmAttraction>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TmAttraction {
    name: Option<String>,
    id: Option<String>,
    classifications: Option<Vec<TmClassification>>,
    externalLinks: Option<TmExternalLinks>,
    images: Option<Vec<Images>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TmExternalLinks {
    wiki: Option<Vec<TmExternalLink>>,
    homepage: Option<Vec<TmExternalLink>>,
    instagram: Option<Vec<TmExternalLink>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TmExternalLink {
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TmClassification {
    primary: Option<bool>,
    segment: Option<TmSegment>,
    genre: Option<TmSegment>,
    subGenre: Option<TmSegment>,
    subType: Option<TmSegment>,
    family: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct TmSegment {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
struct TmVenue {
    name: Option<String>,
    location: Option<TmLocation>,
    city: Option<TmCity>,
}

#[derive(Debug, Deserialize, Clone)]
struct TmCity {
    name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct TmLocation {
    latitude: Option<String>,
    longitude: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    id: String,
    name: String,
    venue: Option<VenueResponse>,
    images: Vec<Images>,
    dates: Option<String>,
    datesPretty: Option<String>,
    classifications: Option<Vec<TmClassification>>,
    attractions: Option<Vec<TmAttraction>>,
    url: Option<String>,
    priceRanges: Option<Vec<TmPriceRange>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Images {
    ratio: Option<String>,
    url: String,
    width: Option<i32>,
    height: Option<i32>,
    fallback: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
struct VenueResponse {
    name: Option<String>,
    location: Option<LocationResponse>,
    city: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct LocationResponse {
    latitude: Option<String>,
    longitude: Option<String>,
}

pub async fn get_events_by_attraction(
    State(state): State<AppState>,
    Query(query): Query<AttractionEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, (axum::http::StatusCode, String)> {
    let url: String = format!(
        "https://app.ticketmaster.com/discovery/v2/events.json?apikey={}&attractionId={}&sort=date,asc",
        state.ticketmaster_key, query.id,
    );
    let resp = state
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| (axum::http::StatusCode::NOT_FOUND, e.to_string()))?;
    let text = resp
        .text()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("error reading body: {e}")))?;

    let body: TicketmasterResponse = serde_json::from_str(&text)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("serde error: {e:?}")))?;
    let events = body
        .embedded
        .into_iter()
        .flat_map(|e| e.events)
        .map(|e| EventResponse {
            id: e.id,
            name: e.name,
            url: e.url,
            venue: e
                .embedded
                .and_then(|emb| emb.venues)
                .and_then(|vs| vs.last().cloned())
                .map(|v| VenueResponse {
                    name: v.name,
                    location: v.location.map(|loc| LocationResponse {
                        latitude: loc.latitude,
                        longitude: loc.longitude,
                    }),
                    city: v.city.and_then(|c| c.name),
                }),
            images: e.images,
            dates: e.dates.start.dateTime.as_ref().cloned().or_else(|| {
                match (
                    e.dates.start.localDate.as_ref(),
                    e.dates.start.localTime.as_ref(),
                ) {
                    (Some(d), Some(t)) => Some(format!("{}T{}", d, t)),
                    (Some(d), None) => Some(d.clone()),
                    _ => None,
                }
            }),
            datesPretty: e.dates.start.dateTime.as_ref().map(|d| {
                DateTime::parse_from_rfc3339(d)
                    .map(|dt| dt.with_timezone(&Local).format("%B %d").to_string())
                    .unwrap_or_else(|_| d.clone())
            }),
            classifications: e.classifications,
            attractions: None,
            priceRanges: e.priceRanges,
        })
        .collect::<Vec<_>>();
    Ok(Json(events))
}
