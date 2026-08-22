import { z } from "zod"

// Mirrors apps/api/src/events/types.rs field-for-field -- every optional
// field here matches a Rust `Option<T>`, not a frontend assumption about
// what's "always" present. Parsed (not just cast) at the wire boundary in
// providers/eventsProvider.tsx, so a real shape mismatch fails loudly
// there instead of silently dropping fields or throwing deep inside a
// component.
//
// `performerSchema.genres` is the *only* canonical-artist data the stream
// carries eagerly (see `Performer.genres`'s doc comment in
// apps/api/src/events/types.rs for why). The rest
// (artwork, similar artists, display name, provider urls -- the
// `amArtistFullSchema` shape below) is fetched on demand, per performer,
// only for a selected event's detail view -- see apps/web/src/EventDetails.tsx
// and apps/api/src/artists/mod.rs's `get_performer_enrichment`. See
// apps/api/docs/adr/0001-canonical-artist-model.md for why even that
// on-demand fetch can still come back with no match (yet, or at all).

/** Rust's serde serializes `Option::None` as JSON `null` by default (the
 * key is present, not omitted) -- wrapping every optional field in this
 * rather than a bare `.nullish()` means the parsed *type* stays the
 * idiomatic `T | undefined` the rest of the app already expects (e.g.
 * `@workspace/ui`'s `Image` type), while parsing still accepts whatever
 * the wire actually sends (`null`, a real value, or a missing key). */
const optional = <T extends z.ZodTypeAny>(schema: T) =>
  schema.nullish().transform((value) => value ?? undefined)

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

/** A recently-released album/single (within 30 days as of the response --
 * see apps/api/src/artists/lookup.rs's `RECENT_RELEASE_WINDOW`), or absent
 * entirely if the artist's latest known release is older than that, or
 * unknown. See apps/api/src/events/types.rs's `RecentRelease`. */
const recentReleaseSchema = z.object({
  name: z.string(),
  url: optional(z.string()),
  artwork: optional(amArtworkSchema),
})

/** `spotify_url` is on the *full* schema, not `amArtistSchema` above --
 * similar artists only ever come from Apple Music's similar-artists view
 * (see apps/api/src/artists/worker.rs module docs), so they never carry
 * one. Both providers are always searched concurrently regardless of which
 * one ends up winning identity/display fields (see
 * apps/api/src/artists/worker.rs's `resolve_fresh`), so `spotify_url` being
 * absent just means Spotify's own catalog search found no confident match --
 * not that Spotify has no page for them -- see
 * apps/api/src/events/types.rs's `ArtistEnrichment::spotify_url` doc
 * comment. */
const amArtistFullSchema = amArtistSchema.extend({
  spotify_url: optional(z.string()),
  genres: z.array(z.string()),
  similar_artists: z.array(amArtistSchema),
  /** Spotify's 0-100 popularity score -- absent for an artist matched via
   * Apple Music with no concurrent Spotify match, same availability rule
   * as `spotify_url` above. */
  popularity: optional(z.number()),
  recent_release: optional(recentReleaseSchema),
})

/** A team's playoff/postseason implication, derived from its standings
 * seed -- see apps/api/src/sports/types.rs's `PlayoffStatus`. Absent for
 * a league with no standings-derivable playoff line (WNBA, NCAA -- see
 * that type's doc comment) or a team ESPN hasn't seeded yet. */
const playoffStatusSchema = z.enum([
  "inPlayoffPosition",
  "inPlayInPosition",
  "outOfPlayoffPosition",
])

/** One team's row in a conference/division standings table -- see
 * apps/api/src/sports/mod.rs's `StandingEntry`. */
const standingEntrySchema = z.object({
  teamName: z.string(),
  record: z.string(),
  standingPosition: optional(z.number()),
  playoffStatus: optional(playoffStatusSchema),
})

/** A team's current record plus its full conference/division standings
 * table, from the on-demand `/api/sports/enrichment` lookup -- see
 * apps/api/src/sports/mod.rs's `TeamEnrichmentResponse`. `record` is
 * ESPN's own formatted summary (e.g. "60-22", or "53-22-7" for a hockey
 * team's wins-losses-OT-losses) rather than split wins/losses fields,
 * since the shape of "a record" varies by sport -- see
 * apps/api/migrations/007_sports_enrichment.sql. `standings` is the whole
 * group's table (every cached team sharing `groupName`), ordered --
 * empty when `groupName` is absent or nothing else in the group is
 * cached yet, see apps/api/src/sports/db.rs's `get_group_standings`. */
const teamEnrichmentSchema = z.object({
  teamName: z.string(),
  record: z.string(),
  groupName: optional(z.string()),
  /** ESPN's crest for this team, when it has one -- see
   * apps/api/src/sports/client.rs's `pick_logo`. Absent for a handful of
   * smaller programs ESPN has no logo for. */
  logoUrl: optional(z.string()),
  standings: z.array(standingEntrySchema),
  /** This team's own playoff implication -- duplicates what's
   * recoverable from its own row in `standings`, but avoids a find-by-
   * name just to show a one-line summary next to the team header. */
  playoffStatus: optional(playoffStatusSchema),
})

const performerSchema = z.object({
  id: optional(z.string()),
  name: optional(z.string()),
  classifications: optional(z.array(classificationSchema)),
  externalLinks: optional(externalLinksSchema),
  images: optional(z.array(z.unknown())),
  /** Persisted canonical-artist genres, attached backend-side when this
   * performer has been matched -- absent (or empty) when not matched (yet,
   * or at all). See the module doc comment above for why this is the only
   * canonical-artist data the stream carries. */
  genres: optional(z.array(z.string())),
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
  state: optional(z.string()),
  stateCode: optional(z.string()),
  /** Street address (Ticketmaster's `address.line1`) -- absent for venues
   * that don't carry one. */
  address: optional(z.string()),
  postalCode: optional(z.string()),
  /** The venue's own Ticketmaster page, distinct from the event's own
   * `url`. */
  url: optional(z.string()),
  images: z.array(imagesSchema),
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
type TeamEnrichment = z.infer<typeof teamEnrichmentSchema>
type StandingEntry = z.infer<typeof standingEntrySchema>

export {
  eventResponseSchema,
  amArtistFullSchema,
  teamEnrichmentSchema,
  type EventResponse,
  type Performer,
  type Classification,
  type ClassificationSegment,
  type ExternalLink,
  type ExternalLinks,
  type AmArtwork,
  type AmArtist,
  type AmArtistFull,
  type TeamEnrichment,
  type StandingEntry,
}
