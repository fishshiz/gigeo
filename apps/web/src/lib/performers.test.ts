import { describe, expect, it } from "vitest"
import type { EventResponse } from "../hooks/eventsStream"
import { ticketmasterAttractionIds } from "./performers"

function makeEvent(overrides: Partial<EventResponse> = {}): EventResponse {
  return {
    id: "1",
    name: "Test Event",
    images: [],
    source: "ticketmaster",
    ...overrides,
  }
}

describe("ticketmasterAttractionIds", () => {
  it("returns performer ids for a Ticketmaster-sourced event", () => {
    const event = makeEvent({
      source: "ticketmaster",
      performers: [
        { id: "K123", name: "Role Model" },
        { id: "K456", name: "Another Artist" },
      ],
    })

    expect(ticketmasterAttractionIds(event)).toEqual(["K123", "K456"])
  })

  it("returns an empty list for a PredictHQ-sourced event, even with performer ids", () => {
    // A PredictHQ performer's `id` is a PredictHQ entity id, not a
    // Ticketmaster attraction id -- /future-events wouldn't recognize it.
    const event = makeEvent({
      source: "predicthq",
      performers: [{ id: "phq-entity-1", name: "Role Model" }],
    })

    expect(ticketmasterAttractionIds(event)).toEqual([])
  })

  it("returns an empty list when there are no performers", () => {
    expect(
      ticketmasterAttractionIds(makeEvent({ performers: undefined }))
    ).toEqual([])
  })

  it("skips performers with no id", () => {
    const event = makeEvent({
      performers: [{ name: "No Id Here" }, { id: "K789", name: "Has Id" }],
    })

    expect(ticketmasterAttractionIds(event)).toEqual(["K789"])
  })
})
