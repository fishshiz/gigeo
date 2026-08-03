import { useCallback, useEffect, useRef, useState } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import type { GeoJSONFeature } from "mapbox-gl"
import { useSearchProvider } from "./providers/searchProvider"
import { useEventsContext } from "./providers/eventsProvider"
import { useNavigateToLocation } from "./hooks/useNavigateToLocation"
import { useMapInstance } from "./hooks/useMapInstance"
import { useResizeFix } from "./hooks/useResizeFix"
import { useMapCamera } from "./hooks/useMapCamera"
import { useEventLayer } from "./hooks/useEventLayer"
import { useClusterSelection } from "./hooks/useClusterSelection"
import { locationFromFeature } from "./lib/mapbox"
import { DrawerWrapper } from "./Drawer/DrawerWrapper"
import { useIsMobile } from "./providers/Breakpoint"
import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"

const MapWrapper = () => {
  const { selectedCoordinates, setSelectedLocation } = useSearchProvider()
  const navigateToLocation = useNavigateToLocation()
  const [rendered, setRendered] = useState(false)
  useEffect(() => {
    // Marks first-mount completion so later effects can skip their initial
    // run; intentional, not a cascading-render risk here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setRendered(true)
  }, [])

  const {
    eventsByDate,
    selectEvents,
    selectedEvents,
    isStreaming,
    searchRadius,
    radiusExpanded,
  } = useEventsContext()

  const mapRef = useRef<MapboxMap | null>(null)
  const mapContainer = useRef<HTMLDivElement | null>(null)

  const theme =
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? import.meta.env.VITE_MAPBOX_DARK_STYLE
      : import.meta.env.VITE_MAPBOX_LIGHT_STYLE

  // What happens with a geolocated position (reverse-geocoding it, updating
  // search state) isn't a map concern -- useMapInstance just reports the
  // coordinates back via this callback.
  const handleGeolocate = useCallback(
    (coordinates: [number, number]) => {
      navigateToLocation(coordinates)

      fetch(
        `/api/reverse-geocode?longitude=${coordinates[0]}&latitude=${coordinates[1]}`
      )
        .then((res) => res.json())
        .then((data: { features?: GeoJSONFeature[] }) => {
          const feature = data.features?.[0]
          const location = feature && locationFromFeature(feature)
          if (location) {
            setSelectedLocation(location)
          }
        })
        .catch((err) => {
          console.error("Failed to reverse geocode geolocated position", err)
        })
    },
    [navigateToLocation, setSelectedLocation]
  )

  useMapInstance(
    mapRef,
    "map-container",
    theme,
    selectedCoordinates,
    handleGeolocate
  )
  useResizeFix(mapRef, mapContainer)
  const camera = useMapCamera(mapRef)
  useEventLayer(mapRef, eventsByDate, selectedEvents, selectEvents)
  useClusterSelection(mapRef, eventsByDate, selectEvents)

  useEffect(() => {
    // Only flies when there's a selected event with a venue location --
    // matches useEventLayer's icon-highlight guard, since both come from
    // the same original combined effect.
    const venueLocation = selectedEvents[0]?.venue?.location
    if (selectedEvents.length && venueLocation !== undefined) {
      camera.flyToVenue(venueLocation.latitude, venueLocation.longitude)
    }
  }, [selectedEvents, camera])

  useEffect(() => {
    if (!rendered) return
    camera.easeToLocation(selectedCoordinates)
  }, [selectedCoordinates, rendered, camera])

  // Once a search had to widen past the base radius to find anything,
  // zoom out to frame the actual area covered instead of staying zoomed
  // in on a radius that came up empty.
  useEffect(() => {
    if (isStreaming || !radiusExpanded || !searchRadius) return
    camera.fitToRadius(selectedCoordinates, searchRadius)
  }, [isStreaming, radiusExpanded, searchRadius, selectedCoordinates, camera])

  return (
    <>
      <div className="relative flex min-h-0 flex-1">
        {!useIsMobile() && <DrawerWrapper />}
        <div ref={mapContainer} id="map-container" className="h-full w-full" />
      </div>
    </>
  )
}

export { MapWrapper }
