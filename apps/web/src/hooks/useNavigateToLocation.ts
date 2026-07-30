import { useCallback } from "react"
import { useSearchProvider } from "@/providers/searchProvider"
import { useEventsContext } from "@/providers/eventsProvider"

/**
 * Changes the selected coordinates (and optionally the location label)
 * together, resetting the events provider's search-radius state in the
 * same commit.
 *
 * Always use this instead of calling setSelectedCoordinates directly.
 * The radius reset and the coordinate change must land in the same React
 * commit — otherwise the fitBounds effect in MapWrapper can briefly see
 * the *previous* location's expanded search radius paired with the *new*
 * coordinates and zoom out to match it.
 */
export function useNavigateToLocation() {
  const { setSelectedCoordinates, setSelectedLocation } = useSearchProvider()
  const { resetEvents } = useEventsContext()

  return useCallback(
    (coordinates: [number, number], location?: string) => {
      resetEvents()
      setSelectedCoordinates(coordinates)
      if (location !== undefined) {
        setSelectedLocation(location)
      }
    },
    [resetEvents, setSelectedCoordinates, setSelectedLocation]
  )
}
