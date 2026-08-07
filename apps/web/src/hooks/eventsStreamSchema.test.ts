import { describe, expect, it } from "vitest"
import { eventResponseSchema } from "./eventsStreamSchema"

function minimalEvent(overrides: Record<string, unknown> = {}) {
  return {
    id: "1",
    name: "Test Event",
    images: [],
    source: "ticketmaster",
    ...overrides,
  }
}

describe("eventResponseSchema", () => {
  it("parses a minimal valid event", () => {
    const result = eventResponseSchema.safeParse(minimalEvent())
    expect(result.success).toBe(true)
  })

  it("rejects an event missing a required field", () => {
    const withoutId: Record<string, unknown> = minimalEvent()
    delete withoutId.id
    const result = eventResponseSchema.safeParse(withoutId)
    expect(result.success).toBe(false)
  })

  it("rejects an event with an unrecognized source", () => {
    const result = eventResponseSchema.safeParse(
      minimalEvent({ source: "spotify" })
    )
    expect(result.success).toBe(false)
  })

  // Rust's serde serializes `Option::None` as JSON `null` (the key stays
  // present), not an omitted key -- this is the actual wire behavior the
  // normalization exists to handle.
  it("normalizes explicit null on an optional field to undefined", () => {
    const result = eventResponseSchema.safeParse(
      minimalEvent({ dates: null, venue: null, url: null })
    )
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.dates).toBeUndefined()
      expect(result.data.venue).toBeUndefined()
      expect(result.data.url).toBeUndefined()
    }
  })

  it("parses fine when optional keys are omitted entirely", () => {
    const result = eventResponseSchema.safeParse(minimalEvent())
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.performers).toBeUndefined()
      expect(result.data.classifications).toBeUndefined()
    }
  })

  it("parses a classification whose nested segment/genre are null", () => {
    // The real shape of a PredictHQ event with no phq_labels -- see
    // apps/api/src/predicthq/normalize.rs's build_classifications.
    const result = eventResponseSchema.safeParse(
      minimalEvent({
        classifications: [
          {
            primary: true,
            segment: { id: "predicthq-music", name: "Music" },
            genre: null,
            subGenre: null,
            subType: null,
            family: false,
          },
        ],
      })
    )
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.classifications?.[0].genre).toBeUndefined()
    }
  })

  it("parses a performer with enrichment and a null artwork", () => {
    // A canonical artist matched via Spotify with no image found (see
    // apps/api/src/artists/worker.rs) -- artwork is genuinely absent, not
    // just missing from a partial response.
    const result = eventResponseSchema.safeParse(
      minimalEvent({
        performers: [
          {
            id: "K123",
            name: "Role Model",
            enrichment: {
              name: "Role Model",
              id: "sp-1",
              apple_music_url: null,
              spotify_url: "https://open.spotify.com/artist/sp-1",
              artwork: null,
              genres: ["Pop"],
              similar_artists: [],
            },
          },
        ],
      })
    )
    expect(result.success).toBe(true)
    if (result.success) {
      const performer = result.data.performers?.[0]
      expect(performer?.enrichment?.artwork).toBeUndefined()
      expect(performer?.enrichment?.genres).toEqual(["Pop"])
      expect(performer?.enrichment?.apple_music_url).toBeUndefined()
      expect(performer?.enrichment?.spotify_url).toBe(
        "https://open.spotify.com/artist/sp-1"
      )
    }
  })

  it("rejects a performer whose enrichment has the wrong type for a required field", () => {
    // Guards against exactly the kind of backend/frontend drift this
    // schema exists to catch loudly -- id must be a string.
    const result = eventResponseSchema.safeParse(
      minimalEvent({
        performers: [
          {
            enrichment: {
              name: "Role Model",
              id: 12345,
              genres: [],
              similar_artists: [],
            },
          },
        ],
      })
    )
    expect(result.success).toBe(false)
  })
})
