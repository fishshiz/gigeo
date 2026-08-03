import { describe, expect, it } from "vitest"
import { CalendarDate } from "@internationalized/date"
import { dateRangeToApiParams, groupDatesByWeek } from "./dates"

const dates = (count: number) =>
  Array.from({ length: count }, (_, i) => `d${i}`)

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
