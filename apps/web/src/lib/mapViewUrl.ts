import { z } from "zod"
import type { MapView } from "./types"

// Mapbox GL's supported zoom range -- matches what the map itself will
// clamp to, so an out-of-range URL value is rejected here rather than
// silently clamped by Mapbox later.
const mapViewParamsSchema = z.object({
  lat: z.coerce.number().min(-90).max(90),
  lng: z.coerce.number().min(-180).max(180),
  zoom: z.coerce.number().min(0).max(22),
})

/** Parses `lat`/`lng`/`zoom` out of a URL query string. All three must be
 * present and valid -- a partial or malformed set is treated the same as
 * absent, since there's no sensible way to fill in just one missing
 * coordinate. */
export function readMapViewFromSearch(search: string): MapView | undefined {
  const params = new URLSearchParams(search)
  const lat = params.get("lat")
  const lng = params.get("lng")
  const zoom = params.get("zoom")
  if (lat === null || lng === null || zoom === null) return undefined

  const result = mapViewParamsSchema.safeParse({ lat, lng, zoom })
  if (!result.success) return undefined

  return {
    latitude: result.data.lat,
    longitude: result.data.lng,
    zoom: result.data.zoom,
  }
}

/** Writes `mapView` into the current URL's query string via
 * `history.replaceState` -- never `pushState`, so continuous panning
 * doesn't turn the back button into a map-undo stack. Preserves any other
 * query params already present rather than clobbering the whole string.
 * Precision matches what Google Maps links carry: enough to reproduce the
 * view, not enough to imply sub-meter accuracy. */
export function writeMapViewToUrl(view: MapView) {
  try {
    const params = new URLSearchParams(window.location.search)
    params.set("lat", view.latitude.toFixed(6))
    params.set("lng", view.longitude.toFixed(6))
    params.set("zoom", view.zoom.toFixed(2))

    const url = `${window.location.pathname}?${params.toString()}${window.location.hash}`
    window.history.replaceState(null, "", url)
  } catch {
    // window/history unavailable (e.g. a non-browser test environment).
  }
}
