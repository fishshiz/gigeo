//! Converting raw Ticketmaster events into this service's normalized
//! `EventResponse` shape — the one place that still needs to know
//! Ticketmaster's raw shape (see `crate::events` for everything downstream
//! of it, which doesn't).

use super::types::TmEvent;
use crate::events::types::{EventResponse, LocationResponse, VenueResponse};
use chrono::{DateTime, Local};

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

    let performers = e.embedded.and_then(|a| a.attractions);

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
        performers,
        price_ranges: e.price_ranges,
        matched_artist: None,
        matched_via: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticketmaster_stream::types::{EventEmbedded, TmDate, TmDateStart, TmVenue};

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
        assert_eq!(normalized.dates.as_deref(), Some("2026-08-01T15:00:00"));
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
}
