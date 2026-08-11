//! `TicketmasterSource`: this source's `EventSource` adapter. Wraps the
//! per-window pagination loop that used to live inline in
//! `get_concerts_tm_stream` — unchanged behavior, just given a seam.

use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::client::{fetch_tm_page, should_fetch_next};
use super::normalize::normalize_event;
use super::types::PageLimit;
use crate::error::AppError;
use crate::events::dedupe_key;
use crate::events::reconcile::merge_same_source_duplicate;
use crate::events::source::{EventSource, Window};
use crate::events::types::EventResponse;

pub(crate) struct TicketmasterSource {
    client: reqwest::Client,
    api_key: String,
    geo_hash: String,
    radius: u8,
    /// Cross-window dedupe — Ticketmaster returns the same show multiple
    /// times across overlapping date windows with different ids. One
    /// instance per request, same lifetime as the local `seen` this
    /// replaces.
    seen: HashSet<String>,
}

impl TicketmasterSource {
    pub(crate) fn new(
        client: reqwest::Client,
        api_key: String,
        geo_hash: String,
        radius: u8,
    ) -> Self {
        Self {
            client,
            api_key,
            geo_hash,
            radius,
            seen: HashSet::new(),
        }
    }
}

impl EventSource for TicketmasterSource {
    /// Always synchronously ready — `must_wait` is irrelevant here, every
    /// call fully pages through this window before returning.
    async fn events_for_window(
        &mut self,
        window: &Window,
        _must_wait: bool,
    ) -> Result<Option<Vec<EventResponse>>, AppError> {
        let mut page = 0_u32;
        let mut out = Vec::new();

        loop {
            let body = fetch_tm_page(
                &self.client,
                &self.api_key,
                &self.geo_hash,
                self.radius,
                &window.start,
                &window.end,
                page,
            )
            .await?;

            if let Some(meta) = &body.page
                && meta.totalElements >= 1000
            {
                tracing::warn!(
                    start = %window.start,
                    end = %window.end,
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
                let event = normalize_event(raw);
                if !belongs_to_window(&event, window) {
                    // Confirmed live (Denver's "Jutes with Mothica", sitting
                    // exactly at a UTC-midnight window boundary): Ticketmaster's
                    // API returns some boundary-exact events for BOTH adjacent
                    // windows' queries. Without this check, whichever window's
                    // fetch happened to run first would claim it via `seen`
                    // below -- misaligning it from PredictHqSource's own
                    // UTC-day bucketing of the same event (see that module's
                    // docs) and permanently defeating cross-source
                    // reconciliation for that pair, since they'd never be
                    // compared within the same window's reconcile call.
                    // Skipping here lets the *correct* window's query (the one
                    // whose fetch this event's own UTC day actually matches)
                    // claim it instead.
                    continue;
                }
                if self.seen.insert(dedupe_key(&event)) {
                    // `dedupe_key` only catches the exact same Ticketmaster
                    // event reappearing across pagination; a second,
                    // differently-titled TM listing of the same real show
                    // (e.g. "La Luz" vs. "La Luz w/ Spacemoth") still needs
                    // `merge_same_source_duplicate`'s fuzzy venue+day+
                    // performer match to collapse within this window.
                    merge_same_source_duplicate(&mut out, event);
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

        Ok(Some(out))
    }
}

/// Whether `event`'s own UTC calendar day matches `window`'s -- the same
/// axis `PredictHqSource` buckets its own events by (see that module's
/// docs), so an event failing this check belongs to (and will be correctly
/// captured by) the adjacent window whose `calendar_day` actually matches
/// instead.
///
/// `true` when `event.dates` isn't a parseable RFC3339 instant (a bare
/// `YYYY-MM-DD` local-date-only fallback, or a timezone-less local
/// date+time fallback -- see `normalize_event`): neither carries a UTC
/// instant to convert, so there's no boundary-crossing ambiguity to
/// resolve, and the existing behavior (trust whichever window's query
/// returned it) is already correct for them.
fn belongs_to_window(event: &EventResponse, window: &Window) -> bool {
    match utc_calendar_day(event.dates.as_deref()) {
        Some(day) => day == window.calendar_day,
        None => true,
    }
}

fn utc_calendar_day(dates: Option<&str>) -> Option<String> {
    dates
        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
        .map(|dt| dt.with_timezone(&Utc).format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{Source, VenueResponse};

    fn window(calendar_day: &str) -> Window {
        Window {
            start: format!("{calendar_day}T00:00:00Z"),
            end: format!("{calendar_day}T23:59:59Z"),
            calendar_day: calendar_day.to_string(),
        }
    }

    fn event_with_dates(dates: Option<&str>) -> EventResponse {
        EventResponse {
            id: "1".to_string(),
            name: "Event".to_string(),
            venue: Some(VenueResponse {
                name: Some("Venue".to_string()),
                location: None,
                city: None,
                state: None,
                state_code: None,
                address: None,
                postal_code: None,
                url: None,
                images: vec![],
            }),
            images: vec![],
            dates: dates.map(str::to_string),
            dates_pretty: None,
            local_calendar_day: None,
            classifications: None,
            performers: None,
            url: None,
            price_ranges: None,
            matched_artist: None,
            matched_via: None,
            source: Source::Ticketmaster,
            rank: None,
            predicted_attendance: None,
        }
    }

    #[test]
    fn belongs_to_window_true_when_utc_day_matches() {
        let event = event_with_dates(Some("2026-08-14T18:00:00Z"));
        assert!(belongs_to_window(&event, &window("2026-08-14")));
    }

    #[test]
    fn belongs_to_window_false_when_utc_day_is_the_adjacent_day() {
        // The confirmed-live Denver bug: an event sitting exactly at a
        // UTC-midnight window boundary ("Jutes with Mothica", dates
        // "2026-08-14T00:00:00Z") was returned by Ticketmaster's own API
        // for BOTH the "2026-08-13" and "2026-08-14" window queries; without
        // this check, the earlier window claimed it via cross-window `seen`
        // dedup, misaligning it from PredictHqSource's "2026-08-14" bucket
        // (UTC day of the same `dates` value) and permanently defeating
        // cross-source reconciliation for the pair.
        let event = event_with_dates(Some("2026-08-14T00:00:00Z"));
        assert!(!belongs_to_window(&event, &window("2026-08-13")));
        assert!(belongs_to_window(&event, &window("2026-08-14")));
    }

    #[test]
    fn belongs_to_window_true_for_bare_local_date_with_no_time_component() {
        // No instant to convert -- no boundary ambiguity to resolve, so this
        // must not be filtered out of whichever window returned it.
        let event = event_with_dates(Some("2026-08-14"));
        assert!(belongs_to_window(&event, &window("2026-08-14")));
    }

    #[test]
    fn belongs_to_window_true_for_timezone_less_local_datetime_fallback() {
        // normalize_event's local_date+local_time fallback (no offset) --
        // also not RFC3339-parseable, same reasoning as the bare-date case.
        let event = event_with_dates(Some("2026-08-14T20:00:00"));
        assert!(belongs_to_window(&event, &window("2026-08-14")));
    }

    #[test]
    fn belongs_to_window_true_when_dates_is_absent() {
        let event = event_with_dates(None);
        assert!(belongs_to_window(&event, &window("2026-08-14")));
    }
}
