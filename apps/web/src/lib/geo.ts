const MILES_PER_DEGREE_LATITUDE = 69

/** Bounding box (west/south, east/north) for a circle of `radiusMiles`
 * around `center`. Approximate — fine for camera framing, not for
 * distance math. */
export function boundsForRadius(
  center: [number, number],
  radiusMiles: number
): [[number, number], [number, number]] {
  const [lng, lat] = center
  const latDelta = radiusMiles / MILES_PER_DEGREE_LATITUDE
  const cosLat = Math.max(Math.cos((lat * Math.PI) / 180), 0.01)
  const lngDelta = radiusMiles / (MILES_PER_DEGREE_LATITUDE * cosLat)

  return [
    [lng - lngDelta, lat - latDelta],
    [lng + lngDelta, lat + latDelta],
  ]
}
