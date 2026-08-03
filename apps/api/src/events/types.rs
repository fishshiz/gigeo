//! The normalized `Event` shape this service returns to clients, and its
//! constituent types — source-agnostic today by convention only (there's
//! still exactly one source, Ticketmaster), not yet behind a real adapter
//! boundary. That boundary (a `source`/`external_id`/`raw_payload` wrapper
//! and a trait per source) is deliberately deferred until a second source
//! actually exists to design it against — see `ticketmaster_stream::normalize`
//! for the one place that still knows about Ticketmaster's raw shape.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    pub id: String,
    pub name: String,
    pub venue: Option<VenueResponse>,
    pub images: Vec<Images>,
    pub dates: Option<String>,
    #[serde(rename = "datesPretty")]
    pub dates_pretty: Option<String>,
    pub classifications: Option<Vec<Classification>>,
    pub performers: Option<Vec<Performer>>,
    pub url: Option<String>,
    #[serde(rename = "priceRanges")]
    pub price_ranges: Option<Vec<PriceRange>>,
    /// Name of the performer that matched one of the caller's Spotify top
    /// artists, for personalized discovery. `None` when the caller isn't
    /// connected, has no personalization data, or no performer matched.
    #[serde(rename = "matchedArtist")]
    pub matched_artist: Option<String>,
    /// Set only when `matched_artist` matched via Apple Music similar-artist
    /// expansion rather than being one of the caller's own Spotify top
    /// artists — names the top artist (`seed`) that produced the match, so
    /// the frontend can render "similar to X" instead of implying the
    /// caller directly listens to `matched_artist`.
    #[serde(rename = "matchedVia")]
    pub matched_via: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Performer {
    pub name: Option<String>,
    pub id: Option<String>,
    pub classifications: Option<Vec<Classification>>,
    #[serde(rename = "externalLinks")]
    pub external_links: Option<ExternalLinks>,
    pub images: Option<Vec<Images>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalLinks {
    wiki: Option<Vec<ExternalLink>>,
    homepage: Option<Vec<ExternalLink>>,
    instagram: Option<Vec<ExternalLink>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExternalLink {
    url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Classification {
    primary: Option<bool>,
    segment: Option<Segment>,
    genre: Option<Segment>,
    #[serde(rename = "subGenre")]
    sub_genre: Option<Segment>,
    #[serde(rename = "subType")]
    sub_type: Option<Segment>,
    family: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Segment {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriceRange {
    currency: String,
    min: f32,
    max: f32,
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
