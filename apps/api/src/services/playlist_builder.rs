
use crate::state::{AppState};
use crate::error::{AppError};
use crate::ticketmaster_stream::{EventsQuery, EventResponse, TicketmasterResponse, LocationResponse, VenueResponse};
use chrono::{DateTime, Local};
use geohash::{encode, Coord};

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

    let resp = state.client.get(&url).send().await.map_err(AppError::Request)?;

    let status = resp.status();
    let text = resp.text().await.map_err(AppError::Request)?;

    if !status.is_success() {
        return Err(AppError::TicketmasterApi {
            status,
            message: text,
        });
    }

    let body: TicketmasterResponse = serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(format!("Ticketmaster decode error: {e}")))?;

    let events = body
        .embedded
        .into_iter()
        .flat_map(|e| e.events)
        .map(|e| {
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
                dates_pretty: dates_pretty,
                classifications: e.classifications,
                attractions,
                price_ranges: e.price_ranges,
            }
        })
        .fold(
            std::collections::HashMap::new(),
            |mut acc: std::collections::HashMap<String, EventResponse>, event| {
                let venue_id = event.venue.as_ref().and_then(|v| v.name.clone());
                let attraction_id = event
                    .attractions
                    .as_ref()
                    .and_then(|attractions| attractions.first().and_then(|a| a.id.clone()));

                let key = format!(
                    "{}-{}-{}",
                    event.dates.clone().unwrap_or_default(),
                    venue_id.unwrap_or_default(),
                    attraction_id.unwrap_or_default()
                );

                acc.entry(key).or_insert(event);
                acc
            },
        )
        .into_values()
        .collect();

    Ok(events)
}


