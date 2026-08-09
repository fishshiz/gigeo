//! `PredictHqSource`: this source's `EventSource` adapter. PredictHQ is
//! fetched once for the whole date range (not per-window — no per-day query
//! cost added by this redesign), so this adapter's job is presenting that
//! single fetch through the same per-window interface Ticketmaster's
//! adapter satisfies: kick the fetch off immediately, answer `None` until
//! it resolves, then serve every subsequent window from an in-memory
//! day-bucketed cache built once, on first successful resolution.
//!
//! Bucketed by the **UTC** calendar day of each event's `dates` field
//! (always a clean UTC rfc3339 string for PredictHQ — see
//! `predicthq::normalize`), matching the same axis `date_windows` already
//! uses to bucket Ticketmaster's own per-window HTTP responses. This is
//! deliberately *not* `EventResponse.local_calendar_day` (venue-local) —
//! that field decides whether two events *match* inside
//! `reconcile_predicthq_events`, not which window's reconcile call an
//! event participates in. Bucketing by the local day instead would misfile
//! an event relative to whichever window Ticketmaster's own UTC-windowed
//! fetch placed its true match in, for exactly the evening-show-crossing-
//! midnight case `events::reconcile`'s
//! `matches_via_local_calendar_day_even_when_utc_dates_disagree` regression
//! test exists to cover.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::task::JoinHandle;

use super::client::{PredictHqEvent, search_concerts};
use super::normalize::normalize_predicthq_event;
use crate::error::AppError;
use crate::events::source::{EventSource, Window};
use crate::events::types::EventResponse;

pub(crate) struct PredictHqSource {
    pending: Option<JoinHandle<Result<Vec<PredictHqEvent>, AppError>>>,
    /// `None` until the fetch resolves; `Some` (possibly with empty
    /// per-day buckets) from then on.
    cache: Option<HashMap<String, Vec<EventResponse>>>,
}

impl PredictHqSource {
    /// Spawns the one whole-range fetch now — concurrent with
    /// Ticketmaster's windowed fetch, matching the timing
    /// `get_concerts_tm_stream` already used before this redesign. When
    /// `api_key` is `None`, returns a source whose cache is already
    /// resolved to empty, so `events_for_window` always answers
    /// `Some(vec![])` immediately — collapses "PredictHQ not configured"
    /// and "PredictHQ resolves late with nothing" into the same code
    /// path, no special-casing needed in `events::merge`.
    pub(crate) fn spawn(
        client: reqwest::Client,
        api_key: Option<String>,
        within: String,
        start_gte: String,
        start_lte: String,
    ) -> Self {
        match api_key {
            Some(api_key) => {
                let handle = tokio::spawn(async move {
                    search_concerts(&client, &api_key, &within, &start_gte, &start_lte).await
                });
                Self {
                    pending: Some(handle),
                    cache: None,
                }
            }
            None => Self {
                pending: None,
                cache: Some(HashMap::new()),
            },
        }
    }

    /// Awaits the spawned fetch (only ever called once `pending` is
    /// confirmed either finished or being force-awaited) and builds the
    /// day-bucketed cache. A failed/absent fetch degrades to "no
    /// PredictHQ data this request" rather than an error — this feature
    /// is additive and must never take down event browsing.
    async fn resolve(&mut self) {
        let Some(handle) = self.pending.take() else {
            return;
        };

        let raw = match handle.await {
            Ok(Ok(events)) => events,
            Ok(Err(err)) => {
                tracing::warn!(error = %err, "failed to fetch PredictHQ events, skipping");
                Vec::new()
            }
            Err(join_err) => {
                tracing::warn!(error = %join_err, "PredictHQ fetch task failed, skipping");
                Vec::new()
            }
        };

        let mut cache: HashMap<String, Vec<EventResponse>> = HashMap::new();
        for event in raw.into_iter().map(normalize_predicthq_event) {
            let day = event
                .dates
                .as_deref()
                .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
                .map(|dt| dt.with_timezone(&Utc).format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            cache.entry(day).or_default().push(event);
        }

        self.cache = Some(cache);
    }
}

impl EventSource for PredictHqSource {
    async fn events_for_window(
        &mut self,
        window: &Window,
        must_wait: bool,
    ) -> Result<Option<Vec<EventResponse>>, AppError> {
        if self.cache.is_none() {
            let ready = self
                .pending
                .as_ref()
                .is_some_and(|h| must_wait || h.is_finished());
            if !ready {
                return Ok(None);
            }
            self.resolve().await;
        }

        Ok(Some(
            self.cache
                .as_ref()
                .and_then(|c| c.get(&window.calendar_day))
                .cloned()
                .unwrap_or_default(),
        ))
    }
}
