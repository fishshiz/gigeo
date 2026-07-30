//! Fetching a single page of events from the Ticketmaster Discovery API.

use super::types::TicketmasterResponse;
use crate::error::AppError;

pub(super) async fn fetch_tm_page(
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
            status,
            message: body,
        });
    }

    resp.json::<TicketmasterResponse>()
        .await
        .map_err(AppError::from)
}
