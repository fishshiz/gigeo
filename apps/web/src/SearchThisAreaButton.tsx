import { Button } from "@workspace/ui/components/ui/Button"
import { RefreshCw } from "lucide-react"
import { useSearchProvider } from "./providers/searchProvider"
import { useMapViewProvider } from "./providers/mapViewProvider"
import { useEventsContext } from "./providers/eventsProvider"
import { distanceMiles } from "./lib/geo"

// How far the camera has to drift from the last search, relative to the
// radius that search actually needed, before re-searching is worth
// offering. Half the radius: close enough that the previous results
// still mostly cover the visible area, far enough that a meaningful
// chunk of what's on screen wasn't part of that search.
const OFFER_THRESHOLD_RATIO = 0.5

type SearchThisAreaButtonProps = {
  /** Left to the caller (MapWrapper) rather than calling
   * eventsProvider's searchThisArea directly -- MapWrapper also needs to
   * know a search-this-area click happened, to suppress the camera's
   * usual "ease to the search location" animation that would otherwise
   * yank the view away from wherever the user just panned to. */
  onSearchThisArea: (coordinates: [number, number]) => void
}

/** Google Maps' "search this area" button -- appears once the camera has
 * panned away from the last searched location, and re-runs the search at
 * the camera's current position on click. Deliberately keyed off
 * distance from the *search* location (selectedCoordinates), not zoom:
 * zooming in or out doesn't invalidate a radius-based search the way
 * panning away from its center does. */
export function SearchThisAreaButton({
  onSearchThisArea,
}: SearchThisAreaButtonProps) {
  const { selectedCoordinates } = useSearchProvider()
  const { mapView } = useMapViewProvider()
  const { searchRadius, isStreaming } = useEventsContext()

  if (searchRadius === null || isStreaming) return null

  const cameraCoordinates: [number, number] = [
    mapView.longitude,
    mapView.latitude,
  ]
  const distance = distanceMiles(cameraCoordinates, selectedCoordinates)
  const shouldOffer = distance > searchRadius * OFFER_THRESHOLD_RATIO

  if (!shouldOffer) return null

  return (
    <Button
      variant="secondary"
      onPress={() => onSearchThisArea(cameraCoordinates)}
      className="absolute top-4 left-1/2 z-10 w-full min-w-[200px] -translate-x-1/2 touch-manipulation rounded-full bg-(--accent-bg) px-2 py-1 text-[11px] font-medium text-(--text-on-accent) shadow-md max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
    >
      <RefreshCw aria-hidden className="h-4 w-4" />
      Search this area
    </Button>
  )
}
