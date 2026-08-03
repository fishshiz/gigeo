import { type Classification } from "../lib/types"

export type EventsByDate = Record<string, EventResponse[]>

export type EventResponse = {
  id: string
  name: string
  venue?: {
    name?: string
    location?: {
      latitude?: string
      longitude?: string
    }
    city?: string
  } | null
  images: Array<{
    ratio?: string
    url: string
    width?: number
    height?: number
    fallback?: boolean
  }>
  dates?: string | null
  datesPretty?: string | null
  classifications?: Classification[] | null
  performers?: Array<{
    id?: string
    name?: string
  }> | null
  url?: string | null
  priceRanges?: Array<{
    currency: string
    min: number
    max: number
  }> | null
  /** Name of the artist that matched one of the caller's Spotify top
   * artists, for personalized discovery. Absent/null when not applicable. */
  matchedArtist?: string | null
  /** Set only when `matchedArtist` matched via Apple Music similar-artist
   * expansion rather than being one of the caller's own top artists — names
   * the top artist that produced the match. */
  matchedVia?: string | null
}

