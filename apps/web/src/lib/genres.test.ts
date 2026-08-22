import { describe, expect, it } from "vitest"
import { nearbyGenres } from "./genres"
import type { EventResponse, EventsByDate } from "../hooks/eventsStream"

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

describe("nearbyGenres", () => {
  it("returns an empty array when there are no events", () => {
    expect(nearbyGenres({})).toEqual([])
  })

  it("collects genres from every performer across every date bucket", () => {
    const eventsByDate: EventsByDate = {
      "2026-08-01": [
        makeEvent({
          performers: [{ name: "Artist A", genres: ["Rock", "Pop"] }],
        }),
      ],
      "2026-08-02": [
        makeEvent({
          performers: [{ name: "Artist B", genres: ["Jazz"] }],
        }),
      ],
    }
    expect(nearbyGenres(eventsByDate)).toEqual(["Jazz", "Pop", "Rock"])
  })

  it("dedupes repeated genres and sorts the result", () => {
    const eventsByDate: EventsByDate = {
      "2026-08-01": [
        makeEvent({
          performers: [
            { name: "Artist A", genres: ["Rock"] },
            { name: "Artist B", genres: ["Rock", "Metal"] },
          ],
        }),
      ],
    }
    expect(nearbyGenres(eventsByDate)).toEqual(["Metal", "Rock"])
  })

  it("skips performers with no genres and events with no performers", () => {
    const eventsByDate: EventsByDate = {
      "2026-08-01": [
        makeEvent({ performers: [{ name: "No Genres Artist" }] }),
        makeEvent({ performers: undefined }),
      ],
    }
    expect(nearbyGenres(eventsByDate)).toEqual([])
  })
})
