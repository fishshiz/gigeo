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

type Image = {
  ratio: string
  url: string
  width: number
  height: number
  fallback: boolean
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

export type { Attraction, Classification, AmArtistFull, ClassificationSegment }
