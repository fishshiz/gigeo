import { useEffect, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import { useMapViewProvider } from "../providers/mapViewProvider"

/** Records the map camera's settled position into mapViewProvider.
 *
 * Listens for "moveend" only, not continuous move/zoom frames -- Mapbox
 * fires "moveend" once a pan, zoom, fly, or ease comes to rest, which
 * covers zoom-only changes too (e.g. scroll-wheel zoom without panning),
 * so a separate "zoomend" listener would be redundant. Keeping this off
 * the high-frequency "move"/"zoom" events matters because it writes into
 * Context, which re-renders every consumer on each update. */
export function useMapViewSync(mapRef: RefObject<MapboxMap | null>) {
  const { setMapView } = useMapViewProvider()

  useEffect(() => {
    const map = mapRef.current
    if (!map) return

    const handleMoveEnd = () => {
      const center = map.getCenter()
      setMapView({
        latitude: center.lat,
        longitude: center.lng,
        zoom: map.getZoom(),
      })
    }

    map.on("moveend", handleMoveEnd)
    return () => {
      map.off("moveend", handleMoveEnd)
    }
  }, [mapRef, setMapView])
}
