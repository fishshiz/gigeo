import { useEffect, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"

/** Debounced `resize()` call whenever `container` changes size.
 *
 * Not a call-on-every-tick observer: Mapbox recalculates its internal
 * camera transform on each `resize()` call, and the drawer's open/close
 * transition fires this observer continuously as its reserved width
 * animates (~150ms). Resizing the GL canvas that often before it settles
 * produces a visible zoom/scale glitch. Waiting for the container to go
 * quiet avoids it — the canvas just stretches via CSS in the meantime,
 * which is cheap and doesn't touch the camera. */
export function useResizeFix(
  mapRef: RefObject<MapboxMap | null>,
  container: RefObject<HTMLDivElement | null>
) {
  useEffect(() => {
    let settleTimeout: ReturnType<typeof setTimeout> | null = null
    const resizeObserver = new ResizeObserver(() => {
      if (settleTimeout) clearTimeout(settleTimeout)
      settleTimeout = setTimeout(() => {
        mapRef.current?.resize()
      }, 200)
    })
    if (container.current) {
      resizeObserver.observe(container.current)
    }
    return () => {
      if (settleTimeout) clearTimeout(settleTimeout)
      resizeObserver.disconnect()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
}
