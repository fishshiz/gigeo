import { describe, expect, it } from "vitest"
import { matchesClassificationFilter, matchesForYouFilter } from "./eventsProvider"
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

describe("matchesForYouFilter", () => {
  it("passes everything when the For You toggle is off", () => {
    expect(matchesForYouFilter(makeEvent(), false)).toBe(true)
  })

  it("excludes an unmatched event once the For You toggle is on", () => {
    expect(matchesForYouFilter(makeEvent(), true)).toBe(false)
  })

  it("includes a matched event once the For You toggle is on", () => {
    const event = makeEvent({ matchedArtist: "Role Model" })
    expect(matchesForYouFilter(event, true)).toBe(true)
  })
})
