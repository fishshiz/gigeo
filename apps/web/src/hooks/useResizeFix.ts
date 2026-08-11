import { useEffect, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"

// ~1 frame at 60fps. Not requestAnimationFrame: rAF is paused entirely
// while the document is hidden/backgrounded, which would silently stop
// the map from ever resizing in that state; a short timeout keeps firing
// regardless and is close enough to frame-rate for this purpose.
const RESIZE_THROTTLE_MS = 16

/** Throttled `resize()` call whenever `container` changes size.
 *
 * Not a debounce: waiting for the container to go quiet (e.g. ~200ms after
 * the drawer's width animation stops changing) leaves the GL canvas -- a
 * fixed pixel size set by mapbox-gl itself, which does not track its
 * container's CSS size on its own -- stale for the length of the wait. On
 * a container that's *shrinking* that's merely invisible overflow, but on
 * one that's *growing* (e.g. the drawer collapsing) it's a visible dead
 * gap between the stale canvas and the container's now-larger edge.
 *
 * Throttling instead keeps `resize()` within about one frame of the
 * container's true size throughout the animation, not just at the end --
 * so there's never a gap (or a stretched canvas, from an earlier attempt
 * at this fix that forced the canvas to CSS-stretch to its container in
 * the meantime) large enough to notice. Still coalesced to at most once
 * per interval (not once per ResizeObserver entry) so a burst of
 * synchronous layout changes triggers one `resize()`, not many. */
export function useResizeFix(
  mapRef: RefObject<MapboxMap | null>,
  container: RefObject<HTMLDivElement | null>
) {
  useEffect(() => {
    let throttleId: ReturnType<typeof setTimeout> | null = null
    const resizeObserver = new ResizeObserver(() => {
      if (throttleId !== null) return
      throttleId = setTimeout(() => {
        throttleId = null
        mapRef.current?.resize()
      }, RESIZE_THROTTLE_MS)
    })
    if (container.current) {
      resizeObserver.observe(container.current)
    }
    return () => {
      if (throttleId !== null) clearTimeout(throttleId)
      resizeObserver.disconnect()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])
}
