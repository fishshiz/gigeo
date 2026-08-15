import { describe, expect, it } from "vitest"
import type { EventResponse } from "../hooks/eventsStream"
import type { Classification } from "../hooks/eventsStreamSchema"
import { majorLeagueFor, isMajorLeagueMatchup } from "./sports"

function makeEvent(overrides: Partial<EventResponse> = {}): EventResponse {
  return {
    id: "1",
    name: "Test Event",
    images: [],
    source: "ticketmaster",
    ...overrides,
  }
}

function classification(
  segment: string,
  subGenre: string | undefined,
  primary: boolean
): Classification {
  return {
    primary,
    segment: { id: "1", name: segment },
    subGenre: subGenre ? { id: "2", name: subGenre } : undefined,
  }
}

// Mirrors apps/api/src/sports/types.rs's own test suite for
// `matchups_from`/`major_league` -- same gating rules, same reasoning,
// just exercised client-side.

describe("majorLeagueFor", () => {
  it("resolves NBA from an explicitly-marked primary classification", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "NBA", true)],
    })
    expect(majorLeagueFor(event)).toBe("NBA")
  })

  it("is case-insensitive on the segment name", () => {
    const event = makeEvent({
      classifications: [classification("sports", "NFL", true)],
    })
    expect(majorLeagueFor(event)).toBe("NFL")
  })

  it("returns null for a non-major-league sport", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "Tennis", true)],
    })
    expect(majorLeagueFor(event)).toBeNull()
  })

  it("returns null for a non-sports event", () => {
    const event = makeEvent({
      classifications: [classification("Music", undefined, true)],
    })
    expect(majorLeagueFor(event)).toBeNull()
  })

  it("requires an explicitly marked primary classification", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "NHL", false)],
    })
    expect(majorLeagueFor(event)).toBeNull()
  })

  it("returns null when there are no classifications", () => {
    expect(majorLeagueFor(makeEvent({ classifications: undefined }))).toBeNull()
  })
})

describe("isMajorLeagueMatchup", () => {
  it("is true for a major-league event with two named performers", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "MLB", true)],
      performers: [{ name: "Yankees", id: "a1" }, { name: "Red Sox", id: "a2" }],
    })
    expect(isMajorLeagueMatchup(event)).toBe(true)
  })

  it("is false for a single-performer event (a non-game listing)", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "MLB", true)],
      performers: [{ name: "Yankees", id: "a1" }],
    })
    expect(isMajorLeagueMatchup(event)).toBe(false)
  })

  it("is false when performers lack names even if there are two", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "NHL", true)],
      performers: [{ name: "Bruins", id: "a1" }, { id: "a2" }],
    })
    expect(isMajorLeagueMatchup(event)).toBe(false)
  })
})
