import { describe, expect, it } from "vitest"
import { CalendarDate } from "@internationalized/date"
import type { EventResponse } from "../hooks/eventsStream"
import {
  dateRangeToApiParams,
  eventDateKey,
  formatDate,
  formatDateTime,
  formatTime,
  groupDatesByWeek,
} from "./dates"

const dates = (count: number) =>
  Array.from({ length: count }, (_, i) => `d${i}`)

const EASTERN = "America/New_York"

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

describe("eventDateKey", () => {
  // LocalDay: the calendar day an event falls on in the viewer's own
  // timezone. Timezone passed explicitly rather than relying on the
  // runner's own default -- the whole point of these cases is to exercise
  // UTC-vs-local calendar-day boundaries, which only show up in a timezone
  // behind UTC (a CI runner that happens to default to UTC would silently
  // pass a broken implementation, since there'd be no offset to shift
  // across).
  it("returns the YYYY-MM-DD portion of a valid date", () => {
    expect(
      eventDateKey(makeEvent({ dates: "2026-08-01T20:00:00Z" }), EASTERN)
    ).toBe("2026-08-01")
  })

  it("returns 'unknown' when dates is missing", () => {
    expect(eventDateKey(makeEvent({ dates: null }), EASTERN)).toBe("unknown")
  })

  it("returns 'unknown' when dates is not a parseable date", () => {
    expect(eventDateKey(makeEvent({ dates: "not-a-date" }), EASTERN)).toBe(
      "unknown"
    )
  })

  it("buckets a late-evening show under its local calendar day, not the next UTC day", () => {
    // Regression test: an 11pm Eastern show is 3am UTC the *next* day. The
    // previous implementation extracted the UTC calendar day
    // (toISOString().substring(0, 10)), which pushed this into
    // "2026-08-02" even though the show itself is on Aug 1 locally.
    expect(
      eventDateKey(makeEvent({ dates: "2026-08-02T03:00:00Z" }), EASTERN)
    ).toBe("2026-08-01")
  })

  it("does not shift an early show that already falls on the same UTC day", () => {
    // 4pm Eastern -- well clear of the UTC day boundary either way, so
    // this should be unaffected by the local-vs-UTC change.
    expect(
      eventDateKey(makeEvent({ dates: "2026-08-01T20:00:00Z" }), EASTERN)
    ).toBe("2026-08-01")
  })

  it("returns a bare YYYY-MM-DD date unchanged, regardless of timezone", () => {
    // Ticketmaster events with only a localDate (no time) come through as
    // a bare date string -- already the day, as written, with nothing to
    // resolve. Proven independent of timezone by picking one nowhere near
    // America/New_York.
    expect(eventDateKey(makeEvent({ dates: "2026-08-01" }), "Asia/Tokyo")).toBe(
      "2026-08-01"
    )
  })

  it("treats a timezone-less local datetime as already-local, regardless of the target timezone", () => {
    // normalize_event's localDate+localTime fallback produces a string
    // with no "Z"/offset. Routing this through `Date` before reformatting
    // in an arbitrary target timezone would double-convert it -- e.g. a
    // 2am no-zone string reformatted in a different zone than whatever
    // produced it can land on the *previous* day. Reading the date
    // substring directly sidesteps that: proven here by using a timezone
    // (Tokyo, UTC+9) that would expose a double-conversion if one were
    // happening, and confirming the day doesn't move.
    expect(
      eventDateKey(makeEvent({ dates: "2026-08-01T23:30:00" }), "Asia/Tokyo")
    ).toBe("2026-08-01")
  })

  it("defaults to the viewer's own timezone when none is passed", () => {
    expect(eventDateKey(makeEvent({ dates: "2026-08-01T20:00:00Z" }))).toMatch(
      /^\d{4}-\d{2}-\d{2}$/
    )
  })
})

describe("formatDate", () => {
  it("formats a bare YYYY-MM-DD date", () => {
    expect(formatDate("2026-08-01")).toBe("August 1, 2026")
  })
})

describe("formatDateTime", () => {
  it("formats a UTC instant in the given timezone", () => {
    expect(formatDateTime("2026-08-01T23:00:00Z", EASTERN)).toBe(
      "August 1 at 7:00 PM"
    )
  })

  it("falls back to formatDate for a bare date with no time component", () => {
    expect(formatDateTime("2026-08-01", EASTERN)).toBe("August 1, 2026")
  })
})

describe("formatTime", () => {
  it("formats a UTC instant's time-of-day in the given timezone", () => {
    expect(formatTime("2026-08-01T23:00:00Z", EASTERN)).toBe("7:00 PM")
  })

  it("returns null for a bare date with no time component", () => {
    expect(formatTime("2026-08-01", EASTERN)).toBe(null)
  })
})

describe("dateRangeToApiParams", () => {
  // Timezone passed explicitly rather than relying on the runner's own
  // default (same reasoning as eventDateKey's regression tests): the bug
  // only shows up in a timezone behind UTC, so a CI runner defaulting to
  // UTC would silently pass a broken implementation.
  const EASTERN = "America/New_York"

  it("anchors start to local midnight, not UTC midnight", () => {
    // Regression test: the previous implementation did
    // `dateRange.start.toString() + "T00:00:00Z"`, treating the selected
    // date as if it were already UTC midnight. In US Eastern (UTC-4 in
    // August), that's 8pm the *previous* day local time -- so a search
    // starting "August 3" silently began at 8pm on August 2, pulling in
    // any show starting between 8pm and midnight on August 2.
    const { start } = dateRangeToApiParams(
      new CalendarDate(2026, 8, 3),
      new CalendarDate(2026, 8, 10),
      EASTERN
    )
    expect(start).toBe("2026-08-03T04:00:00.000Z")
  })

  it("anchors end to local midnight of the day after the selected end date", () => {
    const { end } = dateRangeToApiParams(
      new CalendarDate(2026, 8, 3),
      new CalendarDate(2026, 8, 10),
      EASTERN
    )
    expect(end).toBe("2026-08-11T04:00:00.000Z")
  })

  it("matches naive UTC-midnight construction when the timezone is UTC", () => {
    const { start, end } = dateRangeToApiParams(
      new CalendarDate(2026, 8, 3),
      new CalendarDate(2026, 8, 3),
      "UTC"
    )
    expect(start).toBe("2026-08-03T00:00:00.000Z")
    expect(end).toBe("2026-08-04T00:00:00.000Z")
  })

  it("defaults to the runner's local timezone when none is passed", () => {
    // Just confirms the default parameter path doesn't throw and returns
    // valid ISO strings -- the timezone-specific behavior above is what's
    // actually under test.
    const { start, end } = dateRangeToApiParams(
      new CalendarDate(2026, 8, 3),
      new CalendarDate(2026, 8, 3)
    )
    expect(() => new Date(start)).not.toThrow()
    expect(new Date(end).getTime()).toBeGreaterThan(new Date(start).getTime())
  })
})

describe("groupDatesByWeek", () => {
  it("returns an empty array for no dates", () => {
    expect(groupDatesByWeek([])).toEqual([])
  })

  it("puts fewer than 7 dates in a single group", () => {
    expect(groupDatesByWeek(dates(5))).toEqual([dates(5)])
  })

  it("puts exactly 7 dates in a single group", () => {
    expect(groupDatesByWeek(dates(7))).toEqual([dates(7)])
  })

  it("starts a new group after 7 dates", () => {
    expect(groupDatesByWeek(dates(8))).toEqual([dates(7), ["d7"]])
  })

  it("keeps every date in its correct week group across 3+ weeks", () => {
    // Regression test: the previous implementation used `acc[idx % 7]` to
    // pick the current group, which only lines up with the group index for
    // the first two weeks. From week 3 onward it silently misfiled dates
    // into earlier groups instead of the current one.
    const groups = groupDatesByWeek(dates(17))

    expect(groups).toEqual([
      ["d0", "d1", "d2", "d3", "d4", "d5", "d6"],
      ["d7", "d8", "d9", "d10", "d11", "d12", "d13"],
      ["d14", "d15", "d16"],
    ])
  })
})
