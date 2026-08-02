//! Raw Ticketmaster API response shapes, and the normalized shapes this
//! service returns to clients.

use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct EventsQuery {
    pub latitude: f64,
    pub longitude: f64,
    pub radius: u8,
    pub start: String,
    pub end: String,
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
    pub price_ranges: Option<Vec<TmPriceRange>>,
    pub name: String,
    #[serde(default)]
    pub images: Vec<Images>,
    pub dates: TmDate,
    pub classifications: Option<Vec<TmClassification>>,
    #[serde(rename = "_embedded")]
    pub embedded: Option<EventEmbedded>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TmPriceRange {
    currency: String,
    min: f32,
    max: f32,
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
    pub attractions: Option<Vec<TmAttraction>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmAttraction {
    pub name: Option<String>,
    pub id: Option<String>,
    pub classifications: Option<Vec<TmClassification>>,
    #[serde(rename = "externalLinks")]
    pub external_links: Option<TmExternalLinks>,
    pub images: Option<Vec<Images>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmExternalLinks {
    wiki: Option<Vec<TmExternalLink>>,
    homepage: Option<Vec<TmExternalLink>>,
    instagram: Option<Vec<TmExternalLink>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmExternalLink {
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmClassification {
    primary: Option<bool>,
    segment: Option<TmSegment>,
    genre: Option<TmSegment>,
    #[serde(rename = "subGenre")]
    sub_genre: Option<TmSegment>,
    #[serde(rename = "subType")]
    sub_type: Option<TmSegment>,
    family: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TmSegment {
    id: String,
    name: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Images {
    ratio: Option<String>,
    url: String,
    width: Option<i32>,
    height: Option<i32>,
    fallback: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    pub id: String,
    pub name: String,
    pub venue: Option<VenueResponse>,
    pub images: Vec<Images>,
    pub dates: Option<String>,
    #[serde(rename = "datesPretty")]
    pub dates_pretty: Option<String>,
    pub classifications: Option<Vec<TmClassification>>,
    pub attractions: Option<Vec<TmAttraction>>,
    pub url: Option<String>,
    #[serde(rename = "priceRanges")]
    pub price_ranges: Option<Vec<TmPriceRange>>,
    /// Name of the attraction that matched one of the caller's Spotify top
    /// artists, for personalized discovery. `None` when the caller isn't
    /// connected, has no personalization data, or no attraction matched.
    #[serde(rename = "matchedArtist")]
    pub matched_artist: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct VenueResponse {
    pub name: Option<String>,
    pub location: Option<LocationResponse>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LocationResponse {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}
