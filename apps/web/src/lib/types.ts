interface TmEvent {
  id: string
  name: string
  venue: Venue
  images: Image[]
  dates: string
  attractions: Attraction[]
}

interface Event extends TmEvent {
  datesPretty: string
}

type Attraction = {
  name: string
  externalLinks?: {
    homepage?: Link[]
    instagram?: Link[]
    wiki?: Link[]
  }
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
}

type Location = {
  latitude: string
  longitude: string
}

export type { TmEvent, Event, GroupedEvents, Attraction, Classification }
