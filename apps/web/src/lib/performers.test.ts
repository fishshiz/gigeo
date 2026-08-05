import { describe, expect, it } from "vitest"
import type { EventResponse } from "../hooks/eventsStream"
import { externalLinksForArtist, ticketmasterAttractionIds } from "./performers"

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

describe("externalLinksForArtist", () => {
  it("matches a performer name case- and whitespace-insensitively", () => {
    const performers: EventResponse["performers"] = [
      {
        name: "  Role Model  ",
        externalLinks: { instagram: [{ url: "https://instagram.com/rolemodel" }] },
      },
    ]

    expect(externalLinksForArtist(performers, "role model")?.instagram?.[0]?.url).toBe(
      "https://instagram.com/rolemodel"
    )
  })

  it("returns undefined when no performer name matches", () => {
    const performers: EventResponse["performers"] = [
      { name: "Someone Else", externalLinks: { wiki: [{ url: "https://en.wikipedia.org/x" }] } },
    ]

    expect(externalLinksForArtist(performers, "Role Model")).toBeUndefined()
  })

  it("returns undefined when performers is missing entirely", () => {
    expect(externalLinksForArtist(undefined, "Role Model")).toBeUndefined()
  })

  it("returns undefined for a matched performer with no external links (e.g. PredictHQ-sourced)", () => {
    const performers: EventResponse["performers"] = [{ name: "Role Model" }]

    expect(externalLinksForArtist(performers, "Role Model")).toBeUndefined()
  })
})
