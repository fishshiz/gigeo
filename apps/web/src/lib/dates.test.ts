import { describe, expect, it } from "vitest"
import { groupDatesByWeek } from "./dates"

const dates = (count: number) =>
  Array.from({ length: count }, (_, i) => `d${i}`)

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
