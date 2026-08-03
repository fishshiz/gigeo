import { useCallback, useMemo, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import { boundsForRadius } from "../lib/geo"

/** Every camera movement the map makes, behind one interface. Three
 * different triggers (selecting an event, changing location, a search
 * widening past its base radius) each used to call `mapRef.current.flyTo`/
 * `easeTo`/`fitBounds` directly -- one module now owns how the camera
 * moves regardless of why. */
export function useMapCamera(mapRef: RefObject<MapboxMap | null>) {
  /** Flies to a venue's coordinates -- a no-op if either is missing or
   * unparseable, which is common: not every event has venue geo data. */
  const flyToVenue = useCallback(
    (latitude: string | undefined, longitude: string | undefined) => {
      if (!latitude || !longitude) return
      const lat = parseFloat(latitude)
      const lng = parseFloat(longitude)
      if (Number.isNaN(lat) || Number.isNaN(lng)) return
      mapRef.current?.flyTo({ center: { lat, lng }, speed: 0.8 })
    },
    [mapRef]
  )

  const easeToLocation = useCallback(
    (coordinates: [number, number]) => {
      mapRef.current?.easeTo({
        center: { lat: coordinates[1], lng: coordinates[0] },
        zoom: 12,
        speed: 0.8,
      })
    },
    [mapRef]
  )

  const fitToRadius = useCallback(
    (coordinates: [number, number], radiusMiles: number) => {
      mapRef.current?.fitBounds(boundsForRadius(coordinates, radiusMiles), {
        padding: 60,
        duration: 800,
      })
    },
    [mapRef]
  )

  // Memoized as a whole so callers can depend on `camera` itself in effect
  // dependency arrays without it changing identity every render.
  return useMemo(
    () => ({ flyToVenue, easeToLocation, fitToRadius }),
    [flyToVenue, easeToLocation, fitToRadius]
  )
}
