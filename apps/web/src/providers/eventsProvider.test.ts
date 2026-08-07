import { describe, expect, it } from "vitest"
import {
  matchedPerformerGenres,
  matchesClassificationFilter,
  matchesForYouFilter,
} from "./eventsProvider"
import type { EventResponse } from "../hooks/eventsStream"

function makeEvent(overrides: Partial<EventResponse> = {}): EventResponse {
  return {
    id: "1",
    name: "Test Event",
    images: [],
    dates: "2026-08-01T20:00:00Z",
    source: "ticketmaster",
    ...overrides,
  }
}

describe("matchesClassificationFilter", () => {
  it("passes everything when no classification is active", () => {
    expect(matchesClassificationFilter(makeEvent(), new Set())).toBe(true)
  })

  it("matches when a primary classification's segment name is active", () => {
    const event = makeEvent({
      classifications: [
        { primary: true, segment: { id: "KZ1", name: "Music" } },
      ],
    })
    expect(matchesClassificationFilter(event, new Set(["Music"]))).toBe(true)
  })

  it("doesn't match a non-primary classification even if the name is active", () => {
    const event = makeEvent({
      classifications: [
        { primary: false, segment: { id: "KZ1", name: "Music" } },
      ],
    })
    expect(matchesClassificationFilter(event, new Set(["Music"]))).toBe(false)
  })

  it("doesn't match when the active set has a different segment name", () => {
    const event = makeEvent({
      classifications: [
        { primary: true, segment: { id: "KZ1", name: "Music" } },
      ],
    })
    expect(matchesClassificationFilter(event, new Set(["Sports"]))).toBe(
      false
    )
  })
})

describe("matchedPerformerGenres", () => {
  it("returns an empty array when the event has no matchedArtist", () => {
    expect(matchedPerformerGenres(makeEvent())).toEqual([])
  })

  it("returns the matched performer's enrichment genres", () => {
    const event = makeEvent({
      matchedArtist: "Role Model",
      performers: [
        {
          name: "Role Model",
          enrichment: {
            name: "Role Model",
            id: "am-1",
            genres: ["Pop", "Indie"],
            similar_artists: [],
          },
        },
      ],
    })
    expect(matchedPerformerGenres(event)).toEqual(["Pop", "Indie"])
  })

  it("returns an empty array when the matched performer has no enrichment yet", () => {
    const event = makeEvent({
      matchedArtist: "Role Model",
      performers: [{ name: "Role Model" }],
    })
    expect(matchedPerformerGenres(event)).toEqual([])
  })

  it("returns an empty array when no performer name matches matchedArtist", () => {
    const event = makeEvent({
      matchedArtist: "Role Model",
      performers: [{ name: "Someone Else" }],
    })
    expect(matchedPerformerGenres(event)).toEqual([])
  })
})

describe("matchesForYouFilter", () => {
  it("passes everything when no For You filter is active", () => {
    expect(matchesForYouFilter(makeEvent(), new Set(), new Set())).toBe(true)
  })

  it("excludes an unmatched event once a For You filter is active", () => {
    expect(
      matchesForYouFilter(makeEvent(), new Set(["Role Model"]), new Set())
    ).toBe(false)
  })

  it("matches on the selected artist", () => {
    const event = makeEvent({ matchedArtist: "Role Model" })
    expect(
      matchesForYouFilter(event, new Set(["Role Model"]), new Set())
    ).toBe(true)
  })

  it("doesn't match a different selected artist", () => {
    const event = makeEvent({ matchedArtist: "Role Model" })
    expect(
      matchesForYouFilter(event, new Set(["Someone Else"]), new Set())
    ).toBe(false)
  })

  it("matches on a selected genre from the matched performer's enrichment", () => {
    const event = makeEvent({
      matchedArtist: "Role Model",
      performers: [
        {
          name: "Role Model",
          enrichment: {
            name: "Role Model",
            id: "am-1",
            genres: ["Pop"],
            similar_artists: [],
          },
        },
      ],
    })
    expect(matchesForYouFilter(event, new Set(), new Set(["Pop"]))).toBe(true)
  })

  it("doesn't match a selected genre the matched performer doesn't have", () => {
    const event = makeEvent({
      matchedArtist: "Role Model",
      performers: [
        {
          name: "Role Model",
          enrichment: {
            name: "Role Model",
            id: "am-1",
            genres: ["Pop"],
            similar_artists: [],
          },
        },
      ],
    })
    expect(matchesForYouFilter(event, new Set(), new Set(["Rock"]))).toBe(
      false
    )
  })
})
