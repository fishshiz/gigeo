//! PredictHQ as a second event source — currently just the fetch +
//! normalize path (`client`/`normalize`), not yet wired into
//! `/concerts/stream`. Cross-source dedup against Ticketmaster and the
//! live-stream wiring are a separate, stacked follow-up.

mod client;
mod normalize;

pub(crate) use client::search_concerts;
pub(crate) use normalize::normalize_predicthq_event;
