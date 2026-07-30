import { describe, expect, it } from "vitest"
import { boundsForRadius } from "./geo"

describe("boundsForRadius", () => {
  it("returns a symmetric box centered on the input coordinate", () => {
    const center: [number, number] = [-97.7431, 30.2672] // Austin, TX
    const [[west, south], [east, north]] = boundsForRadius(center, 25)

    const [lng, lat] = center
    expect(west).toBeLessThan(lng)
    expect(east).toBeGreaterThan(lng)
    expect(south).toBeLessThan(lat)
    expect(north).toBeGreaterThan(lat)
    expect(lng - west).toBeCloseTo(east - lng)
    expect(lat - south).toBeCloseTo(north - lat)
  })

  it("scales linearly with radius", () => {
    const center: [number, number] = [0, 0]
    const small = boundsForRadius(center, 10)
    const large = boundsForRadius(center, 20)

    const smallLngDelta = small[1][0] - center[0]
    const largeLngDelta = large[1][0] - center[0]
    expect(largeLngDelta).toBeCloseTo(smallLngDelta * 2)
  })

  it("widens the longitude delta at higher latitudes", () => {
    // A degree of longitude covers less ground the further from the
    // equator you are, so the box must widen in degrees to cover the
    // same radius in miles.
    const equator = boundsForRadius([0, 0], 25)
    const highLatitude = boundsForRadius([0, 60], 25)

    const equatorLngDelta = equator[1][0] - 0
    const highLatitudeLngDelta = highLatitude[1][0] - 0
    expect(highLatitudeLngDelta).toBeGreaterThan(equatorLngDelta)
  })

  it("does not divide by zero near the poles", () => {
    const [[west, south], [east, north]] = boundsForRadius([0, 90], 25)
    expect(Number.isFinite(west)).toBe(true)
    expect(Number.isFinite(east)).toBe(true)
    expect(Number.isFinite(south)).toBe(true)
    expect(Number.isFinite(north)).toBe(true)
  })
})
