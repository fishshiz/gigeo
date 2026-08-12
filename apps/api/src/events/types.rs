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
    /// The venue-local calendar day this event falls on, sourced from each
    /// provider's own local-time field (Ticketmaster's `dates.start.localDate`,
    /// PredictHQ's `start_local`) rather than derived from `dates` (UTC) --
    /// used only for cross-source dedup matching (`events::reconcile`), not
    /// sent to the frontend. See `events::reconcile::calendar_day`.
    #[serde(skip_serializing)]
    pub local_calendar_day: Option<String>,
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
    /// Persisted canonical-artist genres (see
    /// `docs/adr/0001-canonical-artist-model.md`), attached by
    /// `crate::artists::attach_genres` right before an event is sent to a
    /// client — the list/map stream's only eager enrichment need (the "for
    /// you" genre filter, `apps/web/src/providers/eventsProvider.tsx`'s
    /// `matchedPerformerGenres`). Empty when the performer hasn't been
    /// matched to an artist yet (or at all), or has no genre data. The rest
    /// of a performer's canonical-artist data (artwork, similar artists,
    /// display name, provider urls) is deliberately *not* sent here — it's
    /// fetched on demand, only for a selected event's performers, via
    /// `GET /artists/enrichment` (see `crate::artists::get_performer_enrichment`
    /// and `apps/web/src/EventDetails.tsx`).
    #[serde(default)]
    pub genres: Vec<String>,
}

/// Persisted canonical-artist data for one performer — the read-side
/// counterpart of `crate::artists::worker`'s write path. Named and shaped
/// to match what used to come back from the now-retired `GET
/// /apple/artist` on-demand endpoint, so the frontend's existing rendering
/// needed minimal change.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArtistEnrichment {
    pub name: String,
    pub id: String,
    pub apple_music_url: Option<String>,
    /// `None` when Spotify's own catalog search found no confident
    /// exact-normalized-name match for this artist (both providers are
    /// always searched concurrently, regardless of whether Apple Music
    /// also matched — see `artists::worker::resolve_fresh`), not
    /// necessarily an artist Spotify itself has no page for.
    pub spotify_url: Option<String>,
    pub artwork: Option<ArtistArtwork>,
    pub genres: Vec<String>,
    pub similar_artists: Vec<SimilarArtistResponse>,
}

/// Already resolved to a concrete size by `crate::artists::worker`
/// (Apple's own artwork URLs are a `{w}x{h}` template) — unlike the old
/// on-demand endpoint's raw Apple Music response, there's no template
/// substitution left for the frontend to do.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArtistArtwork {
    pub url: String,
    #[serde(rename = "bgColor")]
    pub bg_color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SimilarArtistResponse {
    pub name: String,
    pub id: String,
    pub apple_music_url: Option<String>,
    pub artwork: Option<ArtistArtwork>,
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
    pub state: Option<String>,
    #[serde(rename = "stateCode")]
    pub state_code: Option<String>,
    /// Street address (Ticketmaster's `address.line1`) -- `None` for the
    /// ~28%-ish of venues that don't carry one (same kind of gap PredictHQ
    /// has for `name`, see `predicthq::normalize`).
    pub address: Option<String>,
    #[serde(rename = "postalCode")]
    pub postal_code: Option<String>,
    /// The venue's own Ticketmaster page, when available -- distinct from
    /// `EventResponse::url`, which links the specific event.
    pub url: Option<String>,
    pub images: Vec<Images>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct LocationResponse {
    pub latitude: Option<String>,
    pub longitude: Option<String>,
}
