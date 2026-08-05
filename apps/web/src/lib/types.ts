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

export type { Location }
