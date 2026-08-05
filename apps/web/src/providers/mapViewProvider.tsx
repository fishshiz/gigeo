/* eslint-disable react-refresh/only-export-components */
import * as React from "react"
import type { MapView } from "../lib/types"
import { DEFAULT_CENTER } from "./searchProvider"
import { readMapViewFromSearch, writeMapViewToUrl } from "../lib/mapViewUrl"

// Same zoom the camera already settles to after a search (see
// useMapCamera's easeToLocation) -- keeps a first-ever visitor's default
// mapView consistent with what they'd see once the map finishes easing in.
const DEFAULT_ZOOM = 12

const DEFAULT_MAP_VIEW: MapView = {
  latitude: DEFAULT_CENTER[1],
  longitude: DEFAULT_CENTER[0],
  zoom: DEFAULT_ZOOM,
}

const MAP_VIEW_STORAGE_KEY = "gigeo:lastMapView"

function readStoredMapView(): MapView | undefined {
  try {
    const raw = localStorage.getItem(MAP_VIEW_STORAGE_KEY)
    if (!raw) return undefined
    const parsed = JSON.parse(raw)
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      typeof parsed.latitude === "number" &&
      typeof parsed.longitude === "number" &&
      typeof parsed.zoom === "number" &&
      Number.isFinite(parsed.latitude) &&
      Number.isFinite(parsed.longitude) &&
      Number.isFinite(parsed.zoom)
    ) {
      return {
        latitude: parsed.latitude,
        longitude: parsed.longitude,
        zoom: parsed.zoom,
      }
    }
  } catch {
    // localStorage unavailable (e.g. private browsing) or malformed value —
    // fall through to the default.
  }
  return undefined
}

type MapViewProviderState = {
  mapView: MapView
  setMapView: (view: MapView) => void
  /** Whether the initial `mapView` came from a shared link or a
   * remembered session, rather than the app's static default. Consumers
   * use this to skip a first-load "ease in" animation that would
   * otherwise fly away from a deep-linked/restored position back to the
   * default search location. Reflects only the value at mount -- it does
   * not change as the user navigates afterward. */
  isRestoredMapView: boolean
}

type MapViewProviderProps = {
  children: React.ReactNode
}

export const MapViewProviderContext = React.createContext<
  MapViewProviderState | undefined
>(undefined)

export function MapViewProvider({ children }: MapViewProviderProps) {
  // Computed once, at mount, from the URL/localStorage. A ref would also
  // work here but React forbids reading/writing ref values during render
  // (only effects/handlers) -- useMemo with an empty dependency array
  // gets the same "compute once" behavior without tripping that rule.
  const initial = React.useMemo(() => {
    const fromUrl = readMapViewFromSearch(window.location.search)
    if (fromUrl) return { mapView: fromUrl, isRestoredMapView: true }

    const fromStorage = readStoredMapView()
    if (fromStorage) return { mapView: fromStorage, isRestoredMapView: true }

    return { mapView: DEFAULT_MAP_VIEW, isRestoredMapView: false }
  }, [])
  const isRestoredMapView = initial.isRestoredMapView

  const [mapView, setMapView] = React.useState<MapView>(initial.mapView)

  // Remember the last camera position so a returning visitor's map starts
  // roughly where they left it, mirroring searchProvider's coordinate
  // persistence.
  React.useEffect(() => {
    try {
      localStorage.setItem(MAP_VIEW_STORAGE_KEY, JSON.stringify(mapView))
    } catch {
      // Ignore — localStorage may be unavailable (e.g. private browsing).
    }
  }, [mapView])

  // Keeps the address bar shareable at all times, Google-Maps-style --
  // never history.pushState, so panning doesn't turn the back button into
  // a map-undo stack. See writeMapViewToUrl for why.
  React.useEffect(() => {
    writeMapViewToUrl(mapView)
  }, [mapView])

  return (
    <MapViewProviderContext
      value={{
        mapView,
        setMapView,
        isRestoredMapView,
      }}
    >
      {children}
    </MapViewProviderContext>
  )
}

export const useMapViewProvider = () => {
  const context = React.useContext(MapViewProviderContext)

  if (context === undefined) {
    throw new Error("useMapViewProvider must be used within a MapViewProvider")
  }

  return context
}
