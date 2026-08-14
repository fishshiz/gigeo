import { useEffect, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"

/** Collapses the mobile bottom sheet to "peek" when the user taps open map
 * area (not a marker/cluster). An untargeted `click` interaction -- unlike
 * useClusterSelection's/useMapInstance's own targeted ones, this has no
 * `target`, so it would also fire on a marker/cluster tap (interactions
 * are a stack, not automatically mutually exclusive by target) if those
 * handlers didn't already call `event.preventDefault()` to stop it from
 * reaching this one. `enabled` is expected to be `isMobile` -- desktop's
 * sidebar has no snap points to collapse. */
export function useCollapseDrawerOnMapTap(
  mapRef: RefObject<MapboxMap | null>,
  enabled: boolean,
  onCollapse: () => void
) {
  useEffect(() => {
    const map = mapRef.current
    if (!map || !enabled) return

    map.addInteraction("map-tap-collapse-drawer", {
      type: "click",
      handler: () => onCollapse(),
    })

    return () => {
      map.removeInteraction("map-tap-collapse-drawer")
    }
  }, [mapRef, enabled, onCollapse])
}
