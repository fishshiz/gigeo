import { z } from "zod"

// Mirrors apps/api/src/events/types.rs field-for-field -- every optional
// field here matches a Rust `Option<T>`, not a frontend assumption about
// what's "always" present. Parsed (not just cast) at the wire boundary in
// providers/eventsProvider.tsx, so a real shape mismatch fails loudly
// there instead of silently dropping fields or throwing deep inside a
// component. See apps/api/docs/adr/0001-canonical-artist-model.md for why
// `enrichment` below can be absent even for a Music-classified event.

/** Rust's serde serializes `Option::None` as JSON `null` by default (the
 * key is present, not omitted) -- wrapping every optional field in this
 * rather than a bare `.nullish()` means the parsed *type* stays the
 * idiomatic `T | undefined` the rest of the app already expects (e.g.
 * `@workspace/ui`'s `Image` type), while parsing still accepts whatever
 * the wire actually sends (`null`, a real value, or a missing key). */
const optional = <T extends z.ZodTypeAny>(schema: T) =>
  schema
    .nullish()
    .transform((value) => value ?? undefined)

const segmentSchema = z.object({
  id: z.string(),
  name: z.string(),
})

const classificationSchema = z.object({
  primary: optional(z.boolean()),
  segment: optional(segmentSchema),
  genre: optional(segmentSchema),
  subGenre: optional(segmentSchema),
  subType: optional(segmentSchema),
  family: optional(z.boolean()),
})

const externalLinkSchema = z.object({
  url: optional(z.string()),
})

const externalLinksSchema = z.object({
  wiki: optional(z.array(externalLinkSchema)),
  homepage: optional(z.array(externalLinkSchema)),
  instagram: optional(z.array(externalLinkSchema)),
})

/** Already resolved to a concrete size backend-side (Apple's own artwork
 * URLs are a `{w}x{h}` template) -- no substitution left to do here. */
const amArtworkSchema = z.object({
  url: z.string(),
  bgColor: optional(z.string()),
})

/** `artwork`/`apple_music_url` are optional, not just possibly-missing in
 * the UI: a canonical artist matched via Spotify (see
 * apps/api/src/artists/worker.rs) can legitimately have no Apple Music
 * artwork or listen link at all. */
const amArtistSchema = z.object({
  name: z.string(),
  id: z.string(),
  apple_music_url: optional(z.string()),
  artwork: optional(amArtworkSchema),
})

const amArtistFullSchema = amArtistSchema.extend({
  genres: z.array(z.string()),
  similar_artists: z.array(amArtistSchema),
})

const performerSchema = z.object({
  id: optional(z.string()),
  name: optional(z.string()),
  classifications: optional(z.array(classificationSchema)),
  externalLinks: optional(externalLinksSchema),
  images: optional(z.array(z.unknown())),
  /** Persisted canonical-artist data, attached backend-side when this
   * performer has been matched -- absent when not matched (yet, or at
   * all). */
  enrichment: optional(amArtistFullSchema),
})

const imagesSchema = z.object({
  ratio: optional(z.string()),
  url: z.string(),
  width: optional(z.number()),
  height: optional(z.number()),
  fallback: optional(z.boolean()),
})

const locationResponseSchema = z.object({
  latitude: optional(z.string()),
  longitude: optional(z.string()),
})

const venueResponseSchema = z.object({
  name: optional(z.string()),
  location: optional(locationResponseSchema),
  city: optional(z.string()),
})

const priceRangeSchema = z.object({
  currency: z.string(),
  min: z.number(),
  max: z.number(),
})

const eventResponseSchema = z.object({
  id: z.string(),
  name: z.string(),
  venue: optional(venueResponseSchema),
  images: z.array(imagesSchema),
  dates: optional(z.string()),
  datesPretty: optional(z.string()),
  classifications: optional(z.array(classificationSchema)),
  performers: optional(z.array(performerSchema)),
  url: optional(z.string()),
  priceRanges: optional(z.array(priceRangeSchema)),
  /** Name of the performer that matched one of the caller's Spotify top
   * artists, for personalized discovery. Absent when the caller isn't
   * connected, has no personalization data, or no performer matched. */
  matchedArtist: optional(z.string()),
  /** Set only when `matchedArtist` matched via Apple Music similar-artist
   * expansion rather than being one of the caller's own top artists --
   * names the top artist that produced the match. */
  matchedVia: optional(z.string()),
  source: z.enum(["ticketmaster", "predicthq"]),
  /** PredictHQ's 0-100 rank, when available (native PredictHQ events, or a
   * Ticketmaster event enriched via cross-source dedup). */
  rank: optional(z.number()),
  predictedAttendance: optional(z.number()),
})

type EventResponse = z.infer<typeof eventResponseSchema>
type Performer = z.infer<typeof performerSchema>
type Classification = z.infer<typeof classificationSchema>
type ClassificationSegment = z.infer<typeof segmentSchema>
type ExternalLink = z.infer<typeof externalLinkSchema>
type ExternalLinks = z.infer<typeof externalLinksSchema>
type AmArtwork = z.infer<typeof amArtworkSchema>
type AmArtist = z.infer<typeof amArtistSchema>
type AmArtistFull = z.infer<typeof amArtistFullSchema>

export {
  eventResponseSchema,
  type EventResponse,
  type Performer,
  type Classification,
  type ClassificationSegment,
  type ExternalLink,
  type ExternalLinks,
  type AmArtwork,
  type AmArtist,
  type AmArtistFull,
}
