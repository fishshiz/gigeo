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

type AmArtwork = {
  url: string
  width: number
  height: number
  bgColor: string
}

type AmArtist = {
  name: string
  id: string
  apple_music_url: string
  artwork: AmArtwork
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
