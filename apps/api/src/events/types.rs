//! The normalized `Event` shape this service returns to clients, and its
//! constituent types. Two sources feed it now (`ticketmaster_stream` and
//! `predicthq`, each owning the one function that knows its provider's raw
//! shape); `source`/`rank`/`predicted_attendance` below are the fields that
//! became real once a second source did.

use serde::{Deserialize, Serialize};

/// Which provider an event's identity comes from. An event matched across
/// both providers (see the cross-source dedup in `ticketmaster_stream`)
/// keeps `Ticketmaster` as its source — Ticketmaster is the identity,
/// PredictHQ just contributes `rank`/`predicted_attendance` on top of it.
#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Ticketmaster,
    #[serde(rename = "predicthq")]
    PredictHq,
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
    pub source: Source,
    /// PredictHQ's 0-100 rank, when available (native PredictHQ events, or a
    /// Ticketmaster event enriched via cross-source dedup). `None` for a
    /// Ticketmaster event with no PredictHQ match.
    pub rank: Option<u8>,
    /// PredictHQ's predicted attendance figure, under the same availability
    /// rule as `rank`.
    #[serde(rename = "predictedAttendance")]
    pub predicted_attendance: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Performer {
    pub name: Option<String>,
    pub id: Option<String>,
    pub classifications: Option<Vec<Classification>>,
    #[serde(rename = "externalLinks")]
    pub external_links: Option<ExternalLinks>,
    pub images: Option<Vec<Images>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ExternalLinks {
    pub wiki: Option<Vec<ExternalLink>>,
    pub homepage: Option<Vec<ExternalLink>>,
    pub instagram: Option<Vec<ExternalLink>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ExternalLink {
    pub url: Option<String>,
}

/// A single genre/segment tag. Ticketmaster events can carry several
/// (marking one `primary`); PredictHQ's weighted `phq_labels` map onto the
/// same shape, one `Classification` per label (see `predicthq::normalize`).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Classification {
    pub primary: Option<bool>,
    pub segment: Option<Segment>,
    pub genre: Option<Segment>,
    #[serde(rename = "subGenre")]
    pub sub_genre: Option<Segment>,
    #[serde(rename = "subType")]
    pub sub_type: Option<Segment>,
    pub family: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Segment {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PriceRange {
    pub currency: String,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Images {
    pub ratio: Option<String>,
    pub url: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub fallback: Option<bool>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct VenueResponse {
    pub name: Option<String>,
    pub location: Option<LocationResponse>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct LocationResponse {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}
