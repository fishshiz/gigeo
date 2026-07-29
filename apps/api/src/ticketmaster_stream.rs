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
    pub latitude: f64,
    pub longitude: f64,
    pub radius: u8,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize)]
pub struct TicketmasterResponse {
    #[serde(rename = "_embedded")]
    pub embedded: Option<Embedded>,
    pub page: Option<TmPage>,
}

#[derive(Debug, Deserialize)]
pub struct Embedded {
    #[serde(default)]
    pub events: Vec<TmEvent>,
}

#[derive(Debug, Deserialize)]
struct TmPage {
    size: u32,
    totalElements: u32,
    totalPages: u32,
    number: u32,
}

#[derive(Debug, Deserialize)]
pub struct TmEvent {
    pub id: String,
    pub url: Option<String>,
    #[serde(rename = "priceRanges")]
    pub price_ranges: Option<Vec<TmPriceRange>>,
    pub name: String,
    #[serde(default)]
    pub images: Vec<Images>,
    pub dates: TmDate,
    pub classifications: Option<Vec<TmClassification>>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<EventEmbedded>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TmPriceRange {
    currency: String,
    min: f32,
    max: f32,
}

#[derive(Debug, Deserialize)]
pub struct TmDate {
    pub start: TmDateStart,
}

#[derive(Debug, Deserialize)]
pub struct TmDateStart {
    #[serde(default, rename = "dateTime")]
    pub date_time: Option<String>,
    #[serde(default, rename = "localDate")]
    pub local_date: Option<String>,
    #[serde(default, rename = "localTime")]
    pub local_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventEmbedded {
    pub venues: Option<Vec<TmVenue>>,
    pub attractions: Option<Vec<TmAttraction>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmAttraction {
    pub name: Option<String>,
    pub id: Option<String>,
    pub classifications: Option<Vec<TmClassification>>,
    #[serde(rename = "externalLinks")]
    pub external_links: Option<TmExternalLinks>,
    pub images: Option<Vec<Images>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmExternalLinks {
    wiki: Option<Vec<TmExternalLink>>,
    homepage: Option<Vec<TmExternalLink>>,
    instagram: Option<Vec<TmExternalLink>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmExternalLink {
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmClassification {
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
pub struct TmSegment {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmVenue {
    pub name: Option<String>,
    pub location: Option<TmLocation>,
    pub city: Option<TmCity>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmCity {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmLocation {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Images {
    ratio: Option<String>,
    url: String,
    width: Option<i32>,
    height: Option<i32>,
    fallback: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    pub id: String,
    pub name: String,
    pub venue: Option<VenueResponse>,
    pub images: Vec<Images>,
    pub dates: Option<String>,
    #[serde(rename = "datesPretty")]
    pub dates_pretty: Option<String>,
    pub classifications: Option<Vec<TmClassification>>,
    pub attractions: Option<Vec<TmAttraction>>,
    pub url: Option<String>,
    #[serde(rename = "priceRanges")]
    pub price_ranges: Option<Vec<TmPriceRange>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct VenueResponse {
    pub name: Option<String>,
    pub location: Option<LocationResponse>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LocationResponse {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}

pub(crate) fn normalize_event(e: TmEvent) -> EventResponse {
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

pub(crate) fn dedupe_key(event: &EventResponse) -> String {
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
    .map_err(|e| AppError::TicketmasterApi {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("failed to encode geohash: {e}"),
    })?;

    let windows = date_windows(&params.start, &params.end).map_err(|e| AppError::TicketmasterApi {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tm_event(id: &str, start: TmDateStart) -> TmEvent {
        TmEvent {
            id: id.to_string(),
            url: None,
            price_ranges: None,
            name: "Test Event".to_string(),
            images: vec![],
            dates: TmDate { start },
            classifications: None,
            embedded: None,
        }
    }

    #[test]
    fn normalize_event_prefers_date_time() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: Some("2026-08-01T20:00:00Z".to_string()),
                local_date: Some("2026-08-01".to_string()),
                local_time: Some("15:00:00".to_string()),
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates.as_deref(), Some("2026-08-01T20:00:00Z"));
    }

    #[test]
    fn normalize_event_combines_local_date_and_time_when_date_time_missing() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: None,
                local_date: Some("2026-08-01".to_string()),
                local_time: Some("15:00:00".to_string()),
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(
            normalized.dates.as_deref(),
            Some("2026-08-01T15:00:00")
        );
    }

    #[test]
    fn normalize_event_falls_back_to_local_date_only() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: None,
                local_date: Some("2026-08-01".to_string()),
                local_time: None,
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates.as_deref(), Some("2026-08-01"));
    }

    #[test]
    fn normalize_event_has_no_dates_when_all_fields_missing() {
        let event = tm_event(
            "1",
            TmDateStart {
                date_time: None,
                local_date: None,
                local_time: None,
            },
        );
        let normalized = normalize_event(event);
        assert_eq!(normalized.dates, None);
        assert_eq!(normalized.dates_pretty, None);
    }

    #[test]
    fn normalize_event_uses_last_venue_when_multiple_present() {
        let mut event = tm_event(
            "1",
            TmDateStart {
                date_time: Some("2026-08-01T20:00:00Z".to_string()),
                local_date: None,
                local_time: None,
            },
        );
        event.embedded = Some(EventEmbedded {
            venues: Some(vec![
                TmVenue {
                    name: Some("First Venue".to_string()),
                    location: None,
                    city: None,
                },
                TmVenue {
                    name: Some("Second Venue".to_string()),
                    location: None,
                    city: None,
                },
            ]),
            attractions: None,
        });
        let normalized = normalize_event(event);
        assert_eq!(
            normalized.venue.and_then(|v| v.name),
            Some("Second Venue".to_string())
        );
    }

    #[test]
    fn dedupe_key_is_stable_for_same_date_venue_and_attraction() {
        let event = |id: &str| EventResponse {
            id: id.to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some("The Venue".to_string()),
                location: None,
                city: None,
            }),
            images: vec![],
            dates: Some("2026-08-01T20:00:00Z".to_string()),
            dates_pretty: None,
            classifications: None,
            attractions: Some(vec![TmAttraction {
                name: Some("Artist".to_string()),
                id: Some("artist-1".to_string()),
                classifications: None,
                external_links: None,
                images: None,
            }]),
            url: None,
            price_ranges: None,
        };

        // Same date/venue/attraction but a different Ticketmaster event id
        // should still dedupe to the same key — this is the whole point of
        // dedupe_key, since Ticketmaster returns the same show multiple
        // times across overlapping date windows with different ids.
        assert_eq!(dedupe_key(&event("id-a")), dedupe_key(&event("id-b")));
    }

    #[test]
    fn dedupe_key_differs_when_venue_differs() {
        let base = EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some("Venue A".to_string()),
                location: None,
                city: None,
            }),
            images: vec![],
            dates: Some("2026-08-01T20:00:00Z".to_string()),
            dates_pretty: None,
            classifications: None,
            attractions: None,
            url: None,
            price_ranges: None,
        };
        let mut other = EventResponse {
            venue: Some(VenueResponse {
                name: Some("Venue B".to_string()),
                location: None,
                city: None,
            }),
            ..base.clone()
        };
        other.id = "2".to_string();

        assert_ne!(dedupe_key(&base), dedupe_key(&other));
    }

    #[test]
    fn date_windows_single_day_produces_one_window_clipped_to_input() {
        let windows =
            date_windows("2026-08-01T14:00:00Z", "2026-08-01T22:00:00Z").unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].0, "2026-08-01T14:00:00Z");
        assert_eq!(windows[0].1, "2026-08-01T22:00:00Z");
    }

    #[test]
    fn date_windows_splits_multi_day_range_by_calendar_day() {
        let windows =
            date_windows("2026-08-01T14:00:00Z", "2026-08-03T10:00:00Z").unwrap();
        assert_eq!(windows.len(), 3);

        // First window starts at the given start time, not midnight.
        assert_eq!(windows[0].0, "2026-08-01T14:00:00Z");
        assert_eq!(windows[0].1, "2026-08-02T00:00:00Z");

        // Middle window spans the full calendar day.
        assert_eq!(windows[1].0, "2026-08-02T00:00:00Z");
        assert_eq!(windows[1].1, "2026-08-03T00:00:00Z");

        // Last window ends at the given end time, not midnight.
        assert_eq!(windows[2].0, "2026-08-03T00:00:00Z");
        assert_eq!(windows[2].1, "2026-08-03T10:00:00Z");
    }

    #[test]
    fn date_windows_rejects_end_before_start() {
        let result = date_windows("2026-08-03T00:00:00Z", "2026-08-01T00:00:00Z");
        assert!(result.is_err());
    }

    #[test]
    fn date_windows_rejects_malformed_datetime() {
        let result = date_windows("not-a-date", "2026-08-01T00:00:00Z");
        assert!(result.is_err());
    }
}
