import { useEffect, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import type { GeoJSONSource } from "mapbox-gl"
import {
  buildEventFeatureCollection,
  eventIconImageExpression,
} from "../lib/mapbox"
import type { EventResponse, EventsByDate } from "./eventsStream"

/** Owns the "events" Mapbox source/layer end to end: its data (from
 * `eventsByDate`) and its selection-driven icon styling (from
 * `selectedEvents`). These used to be split across two effects that both
 * independently touched the same layer -- keeping them in one hook means
 * one place to change if, say, the clustering or highlight rules change. */
export function useEventLayer(
  mapRef: RefObject<MapboxMap | null>,
  eventsByDate: EventsByDate,
  selectedEvents: EventResponse[],
  selectEvents: (events: EventResponse[]) => void
) {
  useEffect(() => {
    const markEvents = () => {
      selectEvents([])

      const dataSource = buildEventFeatureCollection(eventsByDate)

      // This effect re-runs on every streamed-in event (it depends on
      // eventsByDate, which changes per NDJSON line), so for a city with
      // many results it can fire dozens of times in quick succession —
      // and can fire before the map's style has finished loading, which
      // makes Mapbox throw ("Style is not done loading") on addSource.
      // Kansas-sparse locations rarely hit this race; a dense city like
      // New York reliably does. Defer to the map's own "load" event when
      // the style isn't ready yet, and once a source exists, update its
      // data in place instead of removing/re-adding it every run (which
      // also separately trips an internal Mapbox GL terrain-update bug
      // when done rapidly).
      const applyToMap = () => {
        const existingSource = mapRef.current?.getSource(
          "event-data-source"
        ) as GeoJSONSource | undefined

        if (existingSource) {
          existingSource.setData(dataSource)
          return
        }

        mapRef.current?.addSource("event-data-source", {
          type: "geojson",
          promoteId: "id",
          cluster: true,
          clusterRadius: 0,
          clusterProperties: {
            clusterEvent: ["string", ["get", "description"]],
            clusterVenue: ["string", ["get", "venue"]],
          },
          data: dataSource,
        })
        mapRef.current?.addLayer({
          id: "events",
          type: "symbol",
          source: "event-data-source",
          layout: {
            "text-field": [
              "case",
              ["boolean", ["has", "point_count"], false],
              [
                "concat",
                ["get", "point_count"],
                " events at ",
                ["get", "clusterVenue"],
              ],
              ["get", "description"],
            ],
            "text-variable-anchor": ["top", "bottom", "left", "right"],
            "text-radial-offset": 0.5,
            "text-justify": "auto",
            "icon-image": eventIconImageExpression(selectedEvents),
            "icon-size": 1.6,
          },
          paint: {
            "icon-halo-color": "#ffffff",
            "icon-halo-width": 1,
            "text-color": [
              "case",
              ["boolean", ["feature-state", "hover"], false],
              "#627BC1",
              ["boolean", ["has", "point_count"], false],
              "#373630",
              ["get", "color"],
            ],
            "text-halo-color": "#ffffff",
            "text-halo-width": 2,
          },
        })
      }

      if (mapRef.current?.isStyleLoaded()) {
        applyToMap()
      } else {
        mapRef.current?.once("load", applyToMap)
      }
    }

    markEvents()
    // selectedEvents intentionally omitted: this effect calls selectEvents([])
    // itself (inside markEvents), so including selectedEvents here would
    // make the effect re-trigger the state change that re-runs it — an
    // infinite loop. The effect below already patches icon-image via
    // setLayoutProperty when the selection changes, so rebuilding the whole
    // layer/source here on every click isn't needed either.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventsByDate, selectEvents])

  useEffect(() => {
    // Only updates when there's a selected event with a venue location --
    // matches the original combined effect's guard (which also gated the
    // camera fly-to, now in useMapCamera). Preserved as-is rather than
    // loosened to "any non-empty selection," since that's a slightly
    // different, untested behavior change outside this deepening's scope.
    const venueLocation = selectedEvents[0]?.venue?.location
    if (selectedEvents.length && venueLocation !== undefined) {
      mapRef.current?.setLayoutProperty(
        "events",
        "icon-image",
        eventIconImageExpression(selectedEvents)
      )
    }
  }, [selectedEvents, mapRef])
}
