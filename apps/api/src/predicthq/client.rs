//! PredictHQ Events API client.
//!
//! Endpoint reference: https://docs.predicthq.com/api/events/search-events.md
//!
//! Static bearer-token auth (no OAuth/refresh, unlike Spotify) — architecturally
//! closer to `ticketmaster_stream::client` than to `spotify::client`, so this
//! follows the same free-function-over-a-shared-`reqwest::Client` shape rather
//! than a client struct with methods.

use reqwest::StatusCode;
use serde::Deserialize;

use crate::error::AppError;
use crate::http_utils::request_with_backoff;

const BASE: &str = "https://api.predicthq.com/v1";
/// PredictHQ's own default/max-reasonable page size; matched here so a full
/// `search_concerts` call needs the fewest round-trips.
const PAGE_LIMIT: u32 = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct PredictHqResponse {
    pub(crate) next: Option<String>,
    pub(crate) results: Vec<PredictHqEvent>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PredictHqEvent {
    pub(crate) id: String,
    pub(crate) title: String,
    /// 0-100; PredictHQ's confidence/significance score for the event.
    pub(crate) rank: u8,
    pub(crate) phq_attendance: Option<u32>,
    #[serde(default)]
    pub(crate) entities: Vec<PredictHqEntity>,
    pub(crate) start: String,
    pub(crate) geo: Option<PredictHqGeo>,
    pub(crate) phq_labels: Option<Vec<PhqLabel>>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PredictHqEntity {
    pub(crate) entity_id: String,
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) entity_type: String,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PredictHqGeo {
    pub(crate) geometry: PredictHqGeometry,
    pub(crate) address: Option<PredictHqAddress>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PredictHqGeometry {
    /// `[longitude, latitude]` — GeoJSON's field order, not `(lat, lng)`.
    pub(crate) coordinates: [f64; 2],
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PredictHqAddress {
    pub(crate) locality: Option<String>,
    /// Full street address, e.g. "Fore River Parkway, Portland, ME 04102,
    /// United States of America" — used as a venue-name fallback for the
    /// ~28% of events (confirmed live) with no `venue`-type entity at all.
    pub(crate) formatted_address: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct PhqLabel {
    pub(crate) label: String,
    pub(crate) weight: f64,
}

#[derive(Debug, Deserialize)]
struct PredictHqErrorBody {
    error: String,
}

fn map_error(status: StatusCode, text: String) -> AppError {
    let message = serde_json::from_str::<PredictHqErrorBody>(&text)
        .map(|e| e.error)
        .unwrap_or(text);
    AppError::PredictHqApi { status, message }
}

/// Fetches every concert-category PredictHQ event within `within` (PredictHQ's
/// own radius format, e.g. `"25mi@43.6591,-70.2568"`) starting between
/// `start_gte` and `start_lte` (ISO 8601), following `next` until exhausted.
///
/// Scoped to `category=concerts` only — see the roadmap discussion on why
/// `performing-arts`/`festivals` are deliberately not pulled in yet.
pub(crate) async fn search_concerts(
    client: &reqwest::Client,
    api_key: &str,
    within: &str,
    start_gte: &str,
    start_lte: &str,
) -> Result<Vec<PredictHqEvent>, AppError> {
    let mut events = Vec::new();
    let mut url = format!(
        "{BASE}/events/?category=concerts&within={within}&start.gte={start_gte}&start.lte={start_lte}&limit={PAGE_LIMIT}"
    );

    loop {
        let resp = request_with_backoff(
            "predicthq",
            || client.get(&url).bearer_auth(api_key),
            map_error,
        )
        .await?;

        let page: PredictHqResponse = resp.json().await.map_err(AppError::from)?;
        events.extend(page.results);

        match page.next {
            Some(next_url) => url = next_url,
            None => break,
        }
    }

    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hits the real PredictHQ API — not run by default (needs
    /// `PREDICTHQ_API_KEY` and network access). Run manually with
    /// `cargo test -- --ignored` to confirm the response types still
    /// deserialize real payloads correctly.
    #[tokio::test]
    #[ignore]
    async fn search_concerts_deserializes_real_response() {
        let api_key = std::env::var("PREDICTHQ_API_KEY")
            .expect("PREDICTHQ_API_KEY must be set to run this test");
        let client = reqwest::Client::new();

        let events = search_concerts(
            &client,
            &api_key,
            "25mi@43.6591,-70.2568",
            "2026-08-02",
            "2026-08-09",
        )
        .await
        .expect("search_concerts should succeed against the real API");

        assert!(!events.is_empty(), "expected at least one real event back");
        // At least one event in this market/window is known to carry a
        // structured performer entity and phq_labels — confirms the nested
        // entity/label deserialization isn't just parsing the envelope.
        assert!(
            events.iter().any(|e| e
                .entities
                .iter()
                .any(|en| en.entity_type == "person" || en.entity_type == "organization")),
            "expected at least one event with a person/organization entity"
        );
        assert!(
            events.iter().any(|e| e.phq_labels.is_some()),
            "expected at least one event with phq_labels"
        );
    }

    /// Same real fetch, but through the full normalize path — confirms
    /// `normalize_predicthq_event` doesn't panic on any of the messier real
    /// shapes (missing geo, no entities, no phq_labels) across a real,
    /// heterogeneous 30+ event dataset, not just the hand-built fixtures in
    /// `normalize`'s own unit tests.
    #[tokio::test]
    #[ignore]
    async fn normalizes_every_real_event_without_panicking() {
        use crate::predicthq::normalize_predicthq_event;

        let api_key = std::env::var("PREDICTHQ_API_KEY")
            .expect("PREDICTHQ_API_KEY must be set to run this test");
        let client = reqwest::Client::new();

        let events = search_concerts(
            &client,
            &api_key,
            "25mi@43.6591,-70.2568",
            "2026-08-02",
            "2026-08-09",
        )
        .await
        .expect("search_concerts should succeed against the real API");

        let normalized: Vec<_> = events.into_iter().map(normalize_predicthq_event).collect();

        assert!(
            normalized.iter().any(|e| e.performers.is_none()),
            "expected at least one real event with no structured performer entity"
        );
        assert!(
            normalized
                .iter()
                .any(|e| e.performers.as_ref().is_some_and(|p| p.len() > 1)),
            "expected at least one real event with multiple performer entities"
        );
        assert!(
            normalized
                .iter()
                .any(|e| e.classifications.as_ref().is_some_and(|c| c.len() > 1)),
            "expected at least one real event with multiple phq_labels"
        );
        assert!(
            normalized.iter().all(|e| e.id.starts_with("phq:")),
            "every normalized event should have a source-prefixed id"
        );
        // The fix this test dataset was chosen to exercise: real events
        // with no `venue`-type entity (~28% of this market/window,
        // confirmed live) should still get a usable venue name via
        // `geo.address.formatted_address`, rather than rendering no venue
        // at all.
        assert!(
            normalized.iter().any(|e| e
                .venue
                .as_ref()
                .is_some_and(|v| v.name.as_deref().is_some_and(|n| n.contains(",")))),
            "expected at least one real event whose venue name came from a \
             formatted address fallback (contains a comma), not a venue entity"
        );
    }
}
