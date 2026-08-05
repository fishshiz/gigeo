/** A single Ticketmaster external-link entry -- `url` is optional to match
 * the backend's `Option<String>` rather than assuming every entry
 * Ticketmaster sends is fully populated. */
type ExternalLink = { url?: string | null }

/** A performer's social/wiki/homepage links, as Ticketmaster provides them
 * -- PredictHQ never populates this (see apps/api/src/predicthq/normalize.rs),
 * so it's absent for a PredictHQ-sourced event's performers. */
type ExternalLinks = {
  homepage?: ExternalLink[] | null
  instagram?: ExternalLink[] | null
  wiki?: ExternalLink[] | null
}

type Classification = {
  family: boolean
  primary: boolean
  genre: ClassificationSegment
  subGenre: ClassificationSegment
  segment: ClassificationSegment
  subType: ClassificationSegment
}

type ClassificationSegment = { id: string; name: string }

type Location = {
  fullAddress: string
  cityName: string
  stateCode: string
  countryCode: string
  coordinates: [number, number]
}

/** Already resolved to a concrete size backend-side (Apple's own artwork
 * URLs are a `{w}x{h}` template) -- no substitution left for the frontend
 * to do here. */
type AmArtwork = {
  url: string
  bgColor?: string | null
}

/** `artwork`/`apple_music_url` are optional, not just possibly-missing in
 * the UI: a canonical artist matched via Spotify (see
 * apps/api/src/artists/worker.rs) can legitimately have no Apple Music
 * artwork or listen link at all. */
type AmArtist = {
  name: string
  id: string
  apple_music_url?: string | null
  artwork?: AmArtwork | null
}

interface AmArtistFull extends AmArtist {
  genres: string[]
  similar_artists: AmArtist[]
}

export type {
  ExternalLinks,
  Classification,
  AmArtistFull,
  ClassificationSegment,
  Location,
}
