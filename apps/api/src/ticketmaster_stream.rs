use crate::error::AppError;
use axum::{
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::Response,
};
use bytes::Bytes;
use chrono::{DateTime, Days, Local, NaiveDate, TimeZone, Utc};
use futures::{StreamExt, stream::BoxStream};
use geohash::{Coord, encode};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct EventsQuery {
    latitude: f64,
    longitude: f64,
    radius: u8,
    start: String,
    end: String,
}

#[derive(Debug, Deserialize)]
struct TicketmasterResponse {
    #[serde(rename = "_embedded")]
    embedded: Option<Embedded>,
    page: Option<TmPage>,
}

#[derive(Debug, Deserialize)]
struct Embedded {
    #[serde(default)]
    events: Vec<TmEvent>,
}

#[derive(Debug, Deserialize)]
struct TmPage {
    size: u32,
    totalElements: u32,
    totalPages: u32,
    number: u32,
}

#[derive(Debug, Deserialize)]
struct TmEvent {
    id: String,
    url: Option<String>,
    #[serde(rename = "priceRanges")]
    price_ranges: Option<Vec<TmPriceRange>>,
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
    #[serde(default, rename = "dateTime")]
    date_time: Option<String>,
    #[serde(default, rename = "localDate")]
    local_date: Option<String>,
    #[serde(default, rename = "localTime")]
    local_time: Option<String>,
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
    #[serde(rename = "externalLinks")]
    external_links: Option<TmExternalLinks>,
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
    #[serde(rename = "subGenre")]
    sub_genre: Option<TmSegment>,
    #[serde(rename = "subType")]
    sub_type: Option<TmSegment>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Images {
    ratio: Option<String>,
    url: String,
    width: Option<i32>,
    height: Option<i32>,
    fallback: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    id: String,
    name: String,
    venue: Option<VenueResponse>,
    images: Vec<Images>,
    dates: Option<String>,
    #[serde(rename = "datesPretty")]
    dates_pretty: Option<String>,
    classifications: Option<Vec<TmClassification>>,
    attractions: Option<Vec<TmAttraction>>,
    url: Option<String>,
    #[serde(rename = "priceRanges")]
    price_ranges: Option<Vec<TmPriceRange>>,
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

fn normalize_event(e: TmEvent) -> EventResponse {
    let dates = e.dates.start.date_time.or_else(|| {
        match (
            e.dates.start.local_date.as_ref(),
            e.dates.start.local_time.as_ref(),
        ) {
            (Some(d), Some(t)) => Some(format!("{d}T{t}")),
            (Some(d), None) => Some(d.clone()),
            _ => None,
        }
    });

    let venue = e
        .embedded
        .as_ref()
        .and_then(|emb| emb.venues.as_ref())
        .and_then(|vs| vs.last().cloned());

    let venue = venue.map(|v| VenueResponse {
        name: v.name,
        location: v.location.map(|loc| LocationResponse {
            latitude: loc.latitude,
            longitude: loc.longitude,
        }),
        city: v.city.and_then(|c| c.name),
    });

    let attractions = e.embedded.and_then(|a| a.attractions);

    let dates_pretty = dates.as_ref().map(|d| {
        DateTime::parse_from_rfc3339(d)
            .map(|dt| dt.with_timezone(&Local).format("%B %d").to_string())
            .unwrap_or_else(|_| d.clone())
    });

    EventResponse {
        id: e.id,
        name: e.name,
        url: e.url,
        venue,
        images: e.images,
        dates,
        dates_pretty,
        classifications: e.classifications,
        attractions,
        price_ranges: e.price_ranges,
    }
}

fn dedupe_key(event: &EventResponse) -> String {
    let venue_name = event.venue.as_ref().and_then(|v| v.name.clone());
    let attraction_id = event
        .attractions
        .as_ref()
        .and_then(|xs| xs.first())
        .and_then(|a| a.id.clone());

    format!(
        "{}-{}-{}",
        event.dates.clone().unwrap_or_default(),
        venue_name.unwrap_or_default(),
        attraction_id.unwrap_or_default()
    )
}

fn date_windows(start: &str, end: &str) -> Result<Vec<(String, String)>, String> {
    let start_dt = DateTime::parse_from_rfc3339(start)
        .map_err(|e| format!("invalid start datetime: {e}"))?
        .with_timezone(&Utc);

    let end_dt = DateTime::parse_from_rfc3339(end)
        .map_err(|e| format!("invalid end datetime: {e}"))?
        .with_timezone(&Utc);

    if end_dt < start_dt {
        return Err("end must be >= start".to_string());
    }

    let mut out = Vec::new();
    let mut current: NaiveDate = start_dt.date_naive();
    let last: NaiveDate = end_dt.date_naive();

    while current <= last {
        let day_start = Utc.from_utc_datetime(&current.and_hms_opt(0, 0, 0).unwrap());

        let next_day = current
            .checked_add_days(Days::new(1))
            .ok_or_else(|| "date overflow".to_string())?;

        let day_end = Utc.from_utc_datetime(&next_day.and_hms_opt(0, 0, 0).unwrap());

        let window_start = if current == start_dt.date_naive() {
            start_dt
        } else {
            day_start
        };

        let window_end = if current == end_dt.date_naive() {
            end_dt
        } else {
            day_end
        };

        out.push((
            window_start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            window_end.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        ));

        current = next_day;
    }

    Ok(out)
}

async fn fetch_tm_page(
    client: &reqwest::Client,
    api_key: &str,
    geo_hash: &str,
    radius: u8,
    start: &str,
    end: &str,
    page: u32,
) -> Result<TicketmasterResponse, AppError> {
    let url = format!(
        concat!(
            "https://app.ticketmaster.com/discovery/v2/events.json",
            "?geoPoint={}",
            "&apikey={}",
            "&radius={}",
            "&startDateTime={}",
            "&endDateTime={}",
            "&size=200",
            "&page={}",
            "&sort=date,asc"
        ),
        geo_hash, api_key, radius, start, end, page
    );

    let resp = client.get(&url).send().await.map_err(AppError::from)?;
    let status = resp.status();

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::TicketmasterApi {
            status: status,
            message: body,
        });
    }

    resp.json::<TicketmasterResponse>()
        .await
        .map_err(AppError::from)
}

pub async fn get_concerts_tm_stream(
    State(state): State<AppState>,
    Query(params): Query<EventsQuery>,
) -> Result<Response, AppError> {
    let geo_hash = encode(
        Coord {
            x: params.longitude,
            y: params.latitude,
        },
        6,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to encode geohash: {e}"),
        )
    })
    .unwrap();

    let windows = date_windows(&params.start, &params.end)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
        .unwrap();

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

                if let Some(meta) = &body.page {
                    if meta.totalElements >= 1000 {
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
                }

                let events = body
                    .embedded
                    .map(|embedded| embedded.events)
                    .unwrap_or_default();

                for raw in events {
                    let event = normalize_event(raw);
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
