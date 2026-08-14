import { useCallback, useEffect, useRef, useState } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import type { GeoJSONFeature } from "mapbox-gl"
import { useSearchProvider } from "./providers/searchProvider"
import { useMapViewProvider } from "./providers/mapViewProvider"
import { useEventsContext } from "./providers/eventsProvider"
import { useNavigateToLocation } from "./hooks/useNavigateToLocation"
import { useMapInstance } from "./hooks/useMapInstance"
import { useMapViewSync } from "./hooks/useMapViewSync"
import { useResizeFix } from "./hooks/useResizeFix"
import { useMapCamera } from "./hooks/useMapCamera"
import { useEventLayer } from "./hooks/useEventLayer"
import { useClusterSelection } from "./hooks/useClusterSelection"
import { useCollapseDrawerOnMapTap } from "./hooks/useCollapseDrawerOnMapTap"
import { locationFromFeature } from "./lib/mapbox"
import { DrawerWrapper } from "./Drawer/DrawerWrapper"
import { SearchThisAreaButton } from "./SearchThisAreaButton"
import { useIsMobile } from "./providers/Breakpoint"
import { useDrawerProvider } from "./providers/drawerProvider"
import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"

const MapWrapper = () => {
  const { selectedCoordinates, setSelectedLocation } = useSearchProvider()
  const { mapView, isRestoredMapView } = useMapViewProvider()
  const { setSnapPoint } = useDrawerProvider()
  const navigateToLocation = useNavigateToLocation()
  const [rendered, setRendered] = useState(false)
  useEffect(() => {
    // Marks first-mount completion so later effects can skip their initial
    // run; intentional, not a cascading-render risk here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setRendered(true)
  }, [])

  // Suppresses the next firing of the ease-to-search-location effect
  // below. Set from two places: (1) a shared link or remembered session
  // already has a real camera position, so the very first post-mount
  // firing shouldn't fly away from it back to the search location; (2) a
  // "search this area" click deliberately sets selectedCoordinates to
  // wherever the camera already is, and shouldn't yank the view to a
  // hardcoded zoom in response to its own change. Ref, not state: only
  // needs to gate the next effect firing, never trigger a render itself.
  const skipNextEaseRef = useRef(isRestoredMapView)

  const {
    visibleEventsByDate,
    selectEvents,
    selectedEvents,
    isStreaming,
    searchRadius,
    radiusExpanded,
    searchThisArea,
  } = useEventsContext()

  const handleSearchThisArea = useCallback(
    (coordinates: [number, number]) => {
      skipNextEaseRef.current = true
      fetch(
        `/api/reverse-geocode?longitude=${coordinates[0]}&latitude=${coordinates[1]}`
      )
        .then((res) => res.json())
        .then((data: { features?: GeoJSONFeature[] }) => {
          const feature = data.features?.[0]
          const location = feature && locationFromFeature(feature)
          if (location) {
            setSelectedLocation(location)
            searchThisArea(coordinates)
          }
        })
        .catch((err) => {
          console.error("Failed to reverse geocode geolocated position", err)
        })
    },

    [searchThisArea]
  )

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
    isRestoredMapView
      ? [mapView.longitude, mapView.latitude]
      : selectedCoordinates,
    handleGeolocate,
    isRestoredMapView ? mapView.zoom : undefined
  )
  useResizeFix(mapRef, mapContainer)
  useMapViewSync(mapRef)
  const camera = useMapCamera(mapRef)
  useEventLayer(mapRef, visibleEventsByDate, selectedEvents, selectEvents)
  useClusterSelection(mapRef, visibleEventsByDate, selectEvents)

  const handleCollapseDrawer = useCallback(
    () => setSnapPoint("peek"),
    [setSnapPoint]
  )
  useCollapseDrawerOnMapTap(mapRef, useIsMobile(), handleCollapseDrawer)

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
    if (skipNextEaseRef.current) {
      skipNextEaseRef.current = false
      return
    }
    camera.easeToLocation(selectedCoordinates)
  }, [selectedCoordinates, rendered, camera])

  // Once a search had to widen past the base radius to find anything,
  // zoom out to frame the actual area covered instead of staying zoomed
  // in on a radius that came up empty.
  useEffect(() => {
    if (isStreaming || !searchRadius) return
    camera.fitToRadius(selectedCoordinates, searchRadius)
  }, [isStreaming, radiusExpanded, searchRadius, selectedCoordinates, camera])

  return (
    <>
      <div className="relative flex min-h-0 flex-1">
        {!useIsMobile() && <DrawerWrapper />}
        <div className="relative h-full w-full overflow-hidden">
          <div
            ref={mapContainer}
            id="map-container"
            className="h-full w-full"
          />
          <SearchThisAreaButton onSearchThisArea={handleSearchThisArea} />
          {useIsMobile() && <DrawerWrapper />}
        </div>
      </div>
    </>
  )
}

export { MapWrapper }
