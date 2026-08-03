//! Fetching a single page of events from the Ticketmaster Discovery API,
//! and the pure pagination-limit decision (`should_fetch_next`) shared by
//! every caller that pages through results.

use super::types::{PageLimit, TicketmasterResponse, TmPage};
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

    get_and_parse(client, &url).await
}

/// `GET /discovery/v2/events.json?attractionId=...` — no geo/date filter,
/// since Ticketmaster's attraction lookup doesn't take either.
pub(super) async fn fetch_tm_attraction_page(
    client: &reqwest::Client,
    api_key: &str,
    attraction_id: &str,
    page: u32,
) -> Result<TicketmasterResponse, AppError> {
    let url = format!(
        concat!(
            "https://app.ticketmaster.com/discovery/v2/events.json",
            "?attractionId={}",
            "&apikey={}",
            "&size=200",
            "&page={}",
            "&sort=date,asc"
        ),
        attraction_id, api_key, page
    );

    get_and_parse(client, &url).await
}

async fn get_and_parse(
    client: &reqwest::Client,
    url: &str,
) -> Result<TicketmasterResponse, AppError> {
    let resp = client.get(url).send().await.map_err(AppError::from)?;
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

/// Whether a paginating caller should fetch `page.number + 1` next.
/// `PageLimit::First` never continues; `PageLimit::All` continues until
/// Ticketmaster's `page` metadata says there's nothing left.
pub(super) fn should_fetch_next(limit: PageLimit, page: &TmPage) -> bool {
    match limit {
        PageLimit::First => false,
        PageLimit::All => page.number + 1 < page.totalPages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(number: u32, total_pages: u32) -> TmPage {
        TmPage {
            size: 200,
            totalElements: 0,
            totalPages: total_pages,
            number,
        }
    }

    #[test]
    fn first_never_continues() {
        assert!(!should_fetch_next(PageLimit::First, &page(0, 5)));
        assert!(!should_fetch_next(PageLimit::First, &page(4, 5)));
    }

    #[test]
    fn all_continues_while_pages_remain() {
        assert!(should_fetch_next(PageLimit::All, &page(0, 5)));
        assert!(should_fetch_next(PageLimit::All, &page(3, 5)));
    }

    #[test]
    fn all_stops_on_last_page() {
        assert!(!should_fetch_next(PageLimit::All, &page(4, 5)));
    }

    #[test]
    fn all_stops_on_single_page_result() {
        assert!(!should_fetch_next(PageLimit::All, &page(0, 1)));
    }
}
