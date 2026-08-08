//! The seam between the concert-stream merge/ordering logic (`super::merge`)
//! and where events actually come from. Ticketmaster and PredictHQ are
//! already two adapters for the same conceptual role — fetch concert events
//! for a date range — so this formalizes that as a real interface rather
//! than two bespoke shapes the orchestration has to know about directly.
//! Lets `merge_events` be unit-tested against fake sources with
//! controllable readiness, instead of needing live HTTP.

use crate::error::AppError;
use crate::events::types::EventResponse;

/// One calendar-day query window. `calendar_day` is the UTC day of
/// `start` — the key used to partition PredictHQ's single whole-range
/// fetch into per-window slices (see `predicthq::source` module docs for
/// why this must be the UTC day, not the venue-local day
/// `EventResponse.local_calendar_day` carries).
#[derive(Clone)]
pub(crate) struct Window {
    /// rfc3339, passed straight to Ticketmaster's own windowed fetch.
    pub start: String,
    /// rfc3339.
    pub end: String,
    /// `"YYYY-MM-DD"`, UTC day of `start`.
    pub calendar_day: String,
}

/// A source of concert events for one calendar-day window at a time.
/// Implemented by `ticketmaster_stream::source::TicketmasterSource`
/// (always ready — does its own HTTP pagination synchronously per call)
/// and `predicthq::source::PredictHqSource` (backed by one whole-range
/// background fetch; not ready until that resolves, after which every
/// window is served from an in-memory day-bucketed cache).
pub(crate) trait EventSource {
    /// `Ok(None)` means "not ready yet, ask again on a later window" —
    /// only `PredictHqSource` ever returns it. When `must_wait` is
    /// `true`, the source must block until it can answer definitively
    /// rather than ever return `None` — used exactly once, after every
    /// window has already been requested opportunistically, to flush any
    /// day the source wasn't ready for during the main pass.
    async fn events_for_window(
        &mut self,
        window: &Window,
        must_wait: bool,
    ) -> Result<Option<Vec<EventResponse>>, AppError>;
}
