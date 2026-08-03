import { useEffect, useState, type RefObject } from "react"
import mapboxgl, { type GeoJSONFeature } from "mapbox-gl"

const INITIAL_ZOOM = 1

/** Creates the Mapbox instance and wires up controls/interactions that
 * don't depend on event data: the geolocate control, and cursor changes on
 * hover over the "events" layer.
 *
 * Reports a geolocated position via `onGeolocate` rather than acting on it
 * directly -- what happens with a geolocated position (reverse-geocoding
 * it, updating search state) isn't a map concern, just something the map's
 * control happens to trigger.
 *
 * Takes `mapRef` as a parameter rather than creating and returning its own
 * -- every other map hook needs the same instance, and `MapWrapper` is the
 * single place that owns the ref and wires it into all of them. */
export function useMapInstance(
  mapRef: RefObject<mapboxgl.Map | null>,
  containerId: string,
  style: string,
  initialCenter: [number, number],
  onGeolocate: (coordinates: [number, number]) => void
) {
  const [selectedFeature, setSelectedFeature] = useState<GeoJSONFeature | null>(
    null
  )

  useEffect(() => {
    mapboxgl.accessToken = import.meta.env.VITE_MAPBOX_ACCESS_TOKEN

    mapRef.current = new mapboxgl.Map({
      style,
      container: containerId,
      center: initialCenter,
      zoom: INITIAL_ZOOM,
    })

    mapRef.current.on("load", () => {
      mapRef.current?.resize()
    })

    const geolocate = new mapboxgl.GeolocateControl({
      positionOptions: {
        enableHighAccuracy: false,
      },
      fitBoundsOptions: { maxZoom: INITIAL_ZOOM },
      trackUserLocation: false,
    })

    if (!mapRef.current.hasControl(geolocate)) {
      mapRef.current.addControl(geolocate, "bottom-right")
      geolocate.on("geolocate", (e) => {
        onGeolocate([e.coords.longitude, e.coords.latitude])
      })

      // Only auto-center on the user's real location if permission was
      // already granted in a previous visit — never surprise a first-time
      // visitor with a location prompt before they've done anything. If
      // permission is still undecided or denied, the geolocate control
      // above remains available for the user to trigger explicitly.
      // Some browsers (and this app's own automated preview environment)
      // throw synchronously for unsupported permission queries rather
      // than rejecting the returned promise, so this whole check is
      // wrapped defensively.
      try {
        navigator.permissions
          ?.query({ name: "geolocation" })
          .then((status) => {
            if (status.state === "granted") {
              geolocate.trigger()
            }
          })
          .catch(() => {
            // Permission query unsupported/rejected — fall back to
            // requiring an explicit button click.
          })
      } catch {
        // Permissions API not available at all in this browser.
      }
    }

    mapRef.current?.addInteraction("map-click", {
      type: "click",
      target: { layerId: "events" },
      handler: () => {
        if (selectedFeature) {
          mapRef.current?.setFeatureState(selectedFeature, { selected: false })
          setSelectedFeature(null)
        }
      },
    })
    mapRef.current?.addInteraction("places-mouseenter-interaction", {
      type: "mouseenter",
      target: { layerId: "events" },
      handler: () => {
        if (mapRef.current) {
          mapRef.current.getCanvas().style.cursor = "pointer"
        }
      },
    })
    mapRef.current?.addInteraction("places-mouseleave-interaction", {
      type: "mouseleave",
      target: { layerId: "events" },
      handler: () => {
        if (mapRef.current) {
          mapRef.current.getCanvas().style.cursor = ""
        }
      },
    })

    // Without this, React's Strict Mode double-invoking this effect in
    // development (mount -> unmount -> remount) creates a second Map
    // instance layered on the same container -- the first is orphaned but
    // its canvas is never removed, so it silently sits on top of (and
    // blocks) the real one. Harmless to call on a genuine unmount too,
    // which in production essentially never happens (MapWrapper isn't
    // routed away from), but is the correct lifecycle either way.
    const map = mapRef.current
    return () => {
      map.remove()
      if (mapRef.current === map) {
        mapRef.current = null
      }
    }
    // Intentionally run once: this creates the Mapbox instance itself.
    // Re-running on every dependency change would tear down and recreate
    // the map, not just update it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
}
