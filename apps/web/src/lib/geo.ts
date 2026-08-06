const MILES_PER_DEGREE_LATITUDE = 69
const EARTH_RADIUS_MILES = 3958.8

/** Great-circle distance between two [lng, lat] points, in miles. */
export function distanceMiles(
  a: [number, number],
  b: [number, number]
): number {
  const [lng1, lat1] = a
  const [lng2, lat2] = b
  const toRad = (deg: number) => (deg * Math.PI) / 180

  const dLat = toRad(lat2 - lat1)
  const dLng = toRad(lng2 - lng1)
  const h =
    Math.sin(dLat / 2) ** 2 +
    Math.cos(toRad(lat1)) * Math.cos(toRad(lat2)) * Math.sin(dLng / 2) ** 2

  return 2 * EARTH_RADIUS_MILES * Math.asin(Math.min(1, Math.sqrt(h)))
}

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
