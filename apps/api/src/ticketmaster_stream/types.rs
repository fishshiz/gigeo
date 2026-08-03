//! Raw Ticketmaster API response shapes, and the query/control types this
//! module's handlers accept. The normalized response shape lives in
//! `crate::events::types` — see that module for why.

use crate::events::types::{Classification, Images, Performer, PriceRange};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EventsQuery {
    pub latitude: f64,
    pub longitude: f64,
    pub radius: u8,
    pub start: String,
    pub end: String,
}

#[derive(Deserialize)]
pub struct AttractionEventsQuery {
    pub id: String,
}

/// How many pages of a Ticketmaster response to fetch.
///
/// `First` stops after page 0 — used where completeness matters less than
/// call volume (e.g. `playlist_builder`, which runs on a 5-minute timer).
/// `All` pages through to the end — used where a caller needs every result
/// (the map stream, and attraction lookups where truncating would silently
/// hide an artist's later shows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLimit {
    First,
    All,
}

#[derive(Debug, Deserialize)]
pub struct TicketmasterResponse {
    #[serde(rename = "_embedded")]
    pub embedded: Option<Embedded>,
    pub page: Option<TmPage>,
}

#[derive(Debug, Deserialize)]
pub struct Embedded {
    #[serde(default)]
    pub events: Vec<TmEvent>,
}

#[derive(Debug, Deserialize)]
pub(super) struct TmPage {
    pub(super) size: u32,
    pub(super) totalElements: u32,
    pub(super) totalPages: u32,
    pub(super) number: u32,
}

#[derive(Debug, Deserialize)]
pub struct TmEvent {
    pub id: String,
    pub url: Option<String>,
    #[serde(rename = "priceRanges")]
    pub price_ranges: Option<Vec<PriceRange>>,
    pub name: String,
    #[serde(default)]
    pub images: Vec<Images>,
    pub dates: TmDate,
    pub classifications: Option<Vec<Classification>>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<EventEmbedded>,
}

#[derive(Debug, Deserialize)]
pub struct TmDate {
    pub start: TmDateStart,
}

#[derive(Debug, Deserialize)]
pub struct TmDateStart {
    #[serde(default, rename = "dateTime")]
    pub date_time: Option<String>,
    #[serde(default, rename = "localDate")]
    pub local_date: Option<String>,
    #[serde(default, rename = "localTime")]
    pub local_time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventEmbedded {
    pub venues: Option<Vec<TmVenue>>,
    pub attractions: Option<Vec<Performer>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmVenue {
    pub name: Option<String>,
    pub location: Option<TmLocation>,
    pub city: Option<TmCity>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmCity {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TmLocation {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}
