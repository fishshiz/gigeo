// ExternalLinks/Classification/ClassificationSegment/AmArtistFull are part
// of the event-stream wire shape -- defined once, alongside the zod schema
// that validates them at the boundary, in hooks/eventsStreamSchema.ts.
// Re-exported here so existing imports from "./lib/types" keep working.
export type {
  ExternalLinks,
  Classification,
  AmArtistFull,
  ClassificationSegment,
} from "../hooks/eventsStreamSchema"

type Location = {
  fullAddress: string
  cityName: string
  stateCode: string
  countryCode: string
  coordinates: [number, number]
}

/** The map camera's current position -- distinct from `Location`/
 * `selectedCoordinates`, which track the searched place rather than
 * wherever the user has since panned or zoomed to. */
type MapView = {
  latitude: number
  longitude: number
  zoom: number
}

export type { Location, MapView }
