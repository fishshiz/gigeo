//! PredictHQ as a second event source: fetch + normalize
//! (`client`/`normalize`), plus a best-effort Apple Music image/link
//! backfill (`artwork`) for events that pass through
//! `crate::events::reconcile_predicthq_events` unmatched — PredictHQ's own
//! API provides neither field. Cross-source dedup and the live-stream
//! wiring live in `ticketmaster_stream` and `crate::events`.

mod artwork;
mod client;
mod normalize;

pub(crate) use artwork::backfill_artwork;
pub(crate) use client::search_concerts;
pub(crate) use normalize::normalize_predicthq_event;
