import { useEffect, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import type { GeoJSONSource, InteractionEvent } from "mapbox-gl"
import { resolveEventsByIds } from "../lib/mapbox"
import type { EventResponse, EventsByDate } from "./eventsStream"

/** Wires the click interaction on the "events" layer: resolves whatever
 * was clicked (a single marker, or a cluster) back to the real
 * `EventResponse`s it represents, and selects them. */
export function useClusterSelection(
  mapRef: RefObject<MapboxMap | null>,
  eventsByDate: EventsByDate,
  selectEvents: (events: EventResponse[]) => void
) {
  useEffect(() => {
    const map = mapRef.current
    if (!map) return

    map.addInteraction("event-click-interaction", {
      type: "click",
      target: { layerId: "events" },
      handler: (interactionEvent: InteractionEvent) => {
        // Stops this click from also reaching the untargeted map-tap
        // interaction that collapses the drawer to peek
        // (useCollapseDrawerOnMapTap) -- interactions are a stack, not
        // automatically mutually exclusive by target, so without this a
        // marker/cluster tap would select the event *and* collapse the
        // sheet in the same gesture.
        interactionEvent.preventDefault()
        const { feature } = interactionEvent
        const [event] = resolveEventsByIds(eventsByDate, [feature?.id])
        if (event) {
          selectEvents([event])
        }

        const eventSource = map.getSource("event-data-source") as GeoJSONSource
        eventSource.getClusterChildren(
          feature?.properties.cluster_id as number,
          (error, features) => {
            if (!error) {
              const ids = features?.map((f) => f?.properties?.id) ?? []
              selectEvents(resolveEventsByIds(eventsByDate, ids))
            }
          }
        )
      },
    })

    return () => {
      map.removeInteraction("event-click-interaction")
    }
  }, [eventsByDate, selectEvents, mapRef])
}
