interface TmEvent {
  id: string
  name: string
  venue: Venue
  url?: string
  priceRanges?: {
    currency: string
    max: number
    min: number
  }[]
  images: Image[]
  dates: string
  datesPretty: string
  classifications?: Classification[]
  attractions: Attraction[]
}

interface Event extends TmEvent {}

type Attraction = {
  name: string
  id: string
  externalLinks?: {
    homepage?: Link[]
    instagram?: Link[]
    wiki?: Link[]
  }
  images?: Image[]
  classifications: Classification[]
}

type Link = { url: string }

type Classification = {
  family: boolean
  primary: boolean
  genre: ClassificationSegment
  subGenre: ClassificationSegment
  segment: ClassificationSegment
  subType: ClassificationSegment
}

type ClassificationSegment = { id: string; name: string }

interface GroupedEvents {
  name: string
  grouped: boolean
  venue: string
  dateRange: {
    start: string
    end: string
  }
  attractions: Attraction[]
  events: Event[]
}

type Image = {
  ratio: string
  url: string
  width: number
  height: number
  fallback: boolean
}

type Venue = {
  name: string
  location: Location
  city: string
}

type Location = {
  latitude: string
  longitude: string
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
  TmEvent,
  Event,
  GroupedEvents,
  Attraction,
  Classification,
  AmArtistFull,
  ClassificationSegment,
}
