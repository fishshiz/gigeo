import { type RefObject, useCallback, useEffect, useRef, useState } from "react"

type Options = {
  offsetTop?: number
}

export function useTopMostVisibleInScrollContainer<T extends HTMLElement>(
  containerRef: RefObject<HTMLElement | null>,
  { offsetTop = 0 }: Options = {}
) {
  const elementsRef = useRef(new Map<string, T>())
  const rafRef = useRef<number | null>(null)
  const currentIdRef = useRef<string | null>(null)

  const [topMostId, setTopMostId] = useState<string | null>(null)

  const compute = useCallback(() => {
    const container = containerRef.current
    if (!container) return

    const containerRect = container.getBoundingClientRect()
    const targetTop = containerRect.top + offsetTop

    let bestId: string | null = null
    let bestDistance = Number.POSITIVE_INFINITY

    for (const [id, el] of elementsRef.current) {
      const rect = el.getBoundingClientRect()

      const isVisible =
        rect.bottom > targetTop && rect.top < containerRect.bottom

      if (!isVisible) continue

      const distance = Math.abs(rect.top - targetTop)

      if (distance < bestDistance) {
        bestDistance = distance
        bestId = id
      }
    }

    if (currentIdRef.current !== bestId) {
      currentIdRef.current = bestId
      setTopMostId(bestId)
    }
  }, [containerRef, offsetTop])

  const scheduleCompute = useCallback(() => {
    if (rafRef.current != null) return

    rafRef.current = window.requestAnimationFrame(() => {
      rafRef.current = null
      compute()
    })
  }, [compute])

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const onScroll = () => scheduleCompute()
    const onResize = () => scheduleCompute()

    container.addEventListener("scroll", onScroll, { passive: true })
    window.addEventListener("resize", onResize)

    scheduleCompute()

    return () => {
      container.removeEventListener("scroll", onScroll)
      window.removeEventListener("resize", onResize)

      if (rafRef.current != null) {
        window.cancelAnimationFrame(rafRef.current)
        rafRef.current = null
      }
    }
  }, [containerRef, scheduleCompute])

  const registerItem = useCallback(
    (id: string) => (node: T | null) => {
      if (node) {
        elementsRef.current.set(id, node)
      } else {
        elementsRef.current.delete(id)
      }

      scheduleCompute()
    },
    [scheduleCompute]
  )

  return { topMostId, registerItem, recompute: scheduleCompute }
}
