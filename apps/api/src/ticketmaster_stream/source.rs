//! `TicketmasterSource`: this source's `EventSource` adapter. Wraps the
//! per-window pagination loop that used to live inline in
//! `get_concerts_tm_stream` — unchanged behavior, just given a seam.

use std::collections::HashSet;

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
