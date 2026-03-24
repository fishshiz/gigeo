interface Event {
  id: string
  name: string
  venue: Venue
  images: Image[]
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

export type { Event }
