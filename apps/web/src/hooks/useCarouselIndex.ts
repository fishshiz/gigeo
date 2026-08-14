import { useCallback, useEffect, useRef, useState } from "react"
import type { RefObject } from "react"

/** Tracks which slide of a horizontally snap-scrolling carousel is
 * currently active. Assumes every slide is exactly one container-width
 * wide (e.g. `snap-x snap-mandatory` + `w-full shrink-0` children) --
 * given that shape, the active index is just scrollLeft/clientWidth
 * rounded to the nearest integer, which is simpler and more precise here
 * than an IntersectionObserver (no threshold/rootMargin fuzziness to
 * tune) -- see useTopMostVisibleInScrollContainer (listItemObserver.ts)
 * for the more general version this doesn't need. */
export function useCarouselIndex(
  containerRef: RefObject<HTMLElement | null>
) {
  const [activeIndex, setActiveIndex] = useState(0)
  const rafRef = useRef<number | null>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const compute = () => {
      rafRef.current = null
      const { scrollLeft, clientWidth } = container
      if (clientWidth === 0) return
      setActiveIndex(Math.round(scrollLeft / clientWidth))
    }

    const onScroll = () => {
      if (rafRef.current != null) return
      rafRef.current = window.requestAnimationFrame(compute)
    }

    container.addEventListener("scroll", onScroll, { passive: true })
    return () => {
      container.removeEventListener("scroll", onScroll)
      if (rafRef.current != null) window.cancelAnimationFrame(rafRef.current)
    }
  }, [containerRef])

  const scrollToIndex = useCallback(
    (index: number) => {
      const container = containerRef.current
      if (!container) return
      container.scrollTo({
        left: index * container.clientWidth,
        behavior: "smooth",
      })
    },
    [containerRef]
  )

  return { activeIndex, scrollToIndex }
}
