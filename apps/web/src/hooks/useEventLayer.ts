import { useEffect, useRef, type RefObject } from "react"
import type { Map as MapboxMap } from "mapbox-gl"
import type { GeoJSONSource } from "mapbox-gl"
import {
  buildEventFeatureCollection,
  eventIconColorExpression,
  resolveCssColor,
} from "../lib/mapbox"
import type { EventResponse, EventsByDate } from "./eventsStream"

const MARKER_IMAGE_ID = "marker-pin"
const MARKER_VIEWPORT = 48
const MARKER_PIXEL_RATIO = 2
// A simple pin-with-hole shape (24x24 viewBox) -- generic map iconography,
// not tied to any particular icon set. Fill color here is irrelevant: it's
// loaded as an SDF image (see ensureMarkerImage), so only the alpha shape
// matters, and the actual color comes from the layer's icon-color paint
// property at render time.
const MARKER_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="${MARKER_VIEWPORT}" height="${MARKER_VIEWPORT}" viewBox="0 0 24 24"><path fill="#000" d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5S10.62 6.5 12 6.5s2.5 1.12 2.5 2.5S13.38 11.5 12 11.5z"/></svg>`

/** Registers the app's pin marker as an SDF image on `map`, if not already
 * present. One image, tinted per-feature via icon-color, instead of a
 * separately-baked image per marker color (see eventIconColorExpression). */
function ensureMarkerImage(map: MapboxMap): Promise<void> {
  if (map.hasImage(MARKER_IMAGE_ID)) return Promise.resolve()

  return new Promise((resolve, reject) => {
    const img = new Image(MARKER_VIEWPORT, MARKER_VIEWPORT)
    img.onload = () => {
      if (!map.hasImage(MARKER_IMAGE_ID)) {
        map.addImage(MARKER_IMAGE_ID, img, {
          sdf: true,
          pixelRatio: MARKER_PIXEL_RATIO,
        })
      }
      resolve()
    }
    img.onerror = () => reject(new Error("Failed to load marker icon"))
    img.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(MARKER_SVG)}`
  })
}

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
  // Guards the one-time addSource+addLayer setup below against a race:
  // ensureMarkerImage is async, and this effect can re-run many times in
  // quick succession while events stream in (see the comment on
  // applyToMap) -- without this, several of those re-runs can all
  // observe "no source yet" before the first one's image load resolves,
  // and each then tries to addSource, throwing on every one after the
  // first. Set synchronously the moment setup is claimed, before the
  // async gap opens, so later re-runs within that gap see it and wait
  // instead of racing.
  const layerSetupRef = useRef<Promise<void> | null>(null)

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
        const map = mapRef.current
        if (!map) return

        const existingSource = map.getSource(
          "event-data-source"
        ) as GeoJSONSource | undefined

        if (existingSource) {
          existingSource.setData(dataSource)
          return
        }

        if (!layerSetupRef.current) {
          layerSetupRef.current = ensureMarkerImage(map).then(() => {
            map.addSource("event-data-source", {
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
            map.addLayer({
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
                "icon-image": MARKER_IMAGE_ID,
                "icon-size": 1,
              },
              paint: {
                "icon-color": eventIconColorExpression(
                  selectedEvents,
                  resolveCssColor("--accent-bg"),
                  resolveCssColor("--color-blush-rose-500")
                ),
                "icon-halo-color": "#ffffff",
                "icon-halo-width": 1.5,
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
          })
        } else {
          // Setup already claimed by an earlier, still-in-flight call --
          // this run's data is newer than whatever that one saw, so apply
          // it once setup finishes instead of trying to addSource again.
          layerSetupRef.current.then(() => {
            const source = map.getSource("event-data-source") as
              | GeoJSONSource
              | undefined
            source?.setData(dataSource)
          })
        }
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
    // infinite loop. The effect below already patches icon-color via
    // setPaintProperty when the selection changes, so rebuilding the whole
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
      mapRef.current?.setPaintProperty(
        "events",
        "icon-color",
        eventIconColorExpression(
          selectedEvents,
          resolveCssColor("--accent-bg"),
          resolveCssColor("--color-blush-rose-500")
        )
      )
    }
  }, [selectedEvents, mapRef])
}
