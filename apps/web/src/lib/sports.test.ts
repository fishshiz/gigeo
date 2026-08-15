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

/** A college classification -- `subGenre=College` regardless of sport,
 * `genre` carrying the actual sport, matching what's real live (see
 * `majorLeagueFor`'s doc comment). */
function collegeClassification(genre: string, primary: boolean): Classification {
  return {
    primary,
    segment: { id: "1", name: "Sports" },
    genre: { id: "3", name: genre },
    subGenre: { id: "2", name: "College" },
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

  it("resolves WNBA distinctly from college basketball", () => {
    const event = makeEvent({
      classifications: [classification("Sports", "WNBA", true)],
    })
    expect(majorLeagueFor(event)).toBe("WNBA")
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

  it("resolves NCAA football from a college classification", () => {
    const event = makeEvent({
      classifications: [collegeClassification("Football", true)],
    })
    expect(majorLeagueFor(event)).toBe("NCAA_FOOTBALL")
  })

  it("defaults college basketball to men's when the event name has no gender marker", () => {
    const event = makeEvent({
      name: "Utah State Aggies Mens Basketball vs. Denver Pioneers Mens Basketball",
      classifications: [collegeClassification("Basketball", true)],
    })
    expect(majorLeagueFor(event)).toBe("NCAA_MENS_BASKETBALL")
  })

  it("detects women's college basketball from the event name", () => {
    const event = makeEvent({
      name: "UConn Huskies Women's Basketball vs. Duke Blue Devils Women's Basketball",
      classifications: [collegeClassification("Basketball", true)],
    })
    expect(majorLeagueFor(event)).toBe("NCAA_WOMENS_BASKETBALL")
  })

  it("detects the alternate 'Womens' spelling too", () => {
    const event = makeEvent({
      name: "George Washington Womens Basketball vs. Towson Tigers Womens Basketball",
      classifications: [collegeClassification("Basketball", true)],
    })
    expect(majorLeagueFor(event)).toBe("NCAA_WOMENS_BASKETBALL")
  })

  it("returns null for an uncovered college sport", () => {
    const event = makeEvent({
      classifications: [collegeClassification("Volleyball", true)],
    })
    expect(majorLeagueFor(event)).toBeNull()
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
