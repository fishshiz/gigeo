import { useRef, useEffect, useState } from "react"
import mapboxgl, {
  GeoJSONSource,
  InteractionEvent,
  type GeoJSONFeature,
} from "mapbox-gl"
import { useSearchProvider } from "./providers/searchProvider"
import { useEventsContext } from "./providers/eventsProvider"
import { useNavigateToLocation } from "./hooks/useNavigateToLocation"
import { DrawerWrapper } from "./Drawer/DrawerWrapper"
import { useIsMobile } from "./providers/Breakpoint"
import { boundsForRadius } from "./lib/geo"
import { dateRangeToApiParams } from "./lib/dates"
import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"
import type { Feature, FeatureCollection } from "geojson"
import type { EventResponse } from "./hooks/eventsStream"

const INITIAL_ZOOM = 1

const MapWrapper = () => {
  const { selectedCoordinates, setSelectedLocation, dateRange } =
    useSearchProvider()
  const navigateToLocation = useNavigateToLocation()
  const [rendered, setRendered] = useState(false)
  useEffect(() => {
    // Marks first-mount completion so later effects can skip their initial
    // run; intentional, not a cascading-render risk here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setRendered(true)
  }, [])

  const {
    streamEvents,
    cancelStream,
    eventsByDate,
    selectEvents,
    selectedEvents,
    isStreaming,
    searchRadius,
    radiusExpanded,
  } = useEventsContext()
  const mapRef = useRef<mapboxgl.Map | null>(null)
  const mapContainer = useRef<HTMLDivElement | null>(null)
  const [selectedFeature, setSelectedFeature] = useState(null)
  // Initialize the GeolocateControl.
  const geolocate = new mapboxgl.GeolocateControl({
    positionOptions: {
      enableHighAccuracy: false,
    },
    fitBoundsOptions: { maxZoom: INITIAL_ZOOM },
    trackUserLocation: false,
  })

  useEffect(() => {
    // Debounced rather than calling resize() on every tick: Mapbox
    // recalculates its internal camera transform on each resize() call, and
    // the drawer's open/close transition fires this observer continuously
    // as its reserved width animates (~150ms). Resizing the GL canvas that
    // often before it settles produces a visible zoom/scale glitch. Waiting
    // for the container to go quiet avoids it — the canvas just stretches
    // via CSS in the meantime, which is cheap and doesn't touch the camera.
    let settleTimeout: ReturnType<typeof setTimeout> | null = null
    const resizeObserver = new ResizeObserver(() => {
      if (settleTimeout) clearTimeout(settleTimeout)
      settleTimeout = setTimeout(() => {
        mapRef.current?.resize()
      }, 200)
    })
    if (mapContainer.current) {
      resizeObserver.observe(mapContainer.current)
    }
    return () => {
      if (settleTimeout) clearTimeout(settleTimeout)
      resizeObserver.disconnect()
    }
  }, [])

  useEffect(() => {
    const venueLocation = selectedEvents[0]?.venue?.location
    if (selectedEvents.length && venueLocation !== undefined) {
      const { latitude, longitude } = venueLocation
      mapRef.current?.setLayoutProperty("events", "icon-image", [
        "case",
        ["==", ["get", "id"], selectedEvents[0].id],
        "marker-yellow",
        [
          "in",
          ["get", "clusterVenue"],
          ["literal", selectedEvents.map((e) => e.venue?.name)],
        ],

        "marker-yellow",
        "marker-red",
      ])
      // Selecting an event from the drawer list doesn't necessarily mean
      // the map is already showing its venue (unlike clicking a marker
      // directly, where it's a harmless no-op), so fly there.
      if (latitude && longitude) {
        mapRef.current?.flyTo({
          center: { lat: parseFloat(latitude), lng: parseFloat(longitude) },
          speed: 0.8,
        })
      }
    }
  }, [selectedEvents])

  const theme =
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? import.meta.env.VITE_MAPBOX_DARK_STYLE
      : import.meta.env.VITE_MAPBOX_LIGHT_STYLE
  useEffect(() => {
    mapboxgl.accessToken = import.meta.env.VITE_MAPBOX_ACCESS_TOKEN

    mapRef.current = new mapboxgl.Map({
      style: theme,
      container: "map-container",
      center: selectedCoordinates,
      zoom: INITIAL_ZOOM,
    })

    mapRef.current.on("load", () => {
      mapRef.current?.resize()
    })

    if (!mapRef.current.hasControl(geolocate)) {
      // Add the control to the map.
      mapRef.current.addControl(geolocate, "bottom-right")
      // Set an event listener that fires
      // when a geolocate event occurs.
      geolocate.on("geolocate", (e) => {
        const { longitude, latitude } = e.coords
        navigateToLocation([longitude, latitude])

        fetch(
          `/api/reverse-geocode?longitude=${longitude}&latitude=${latitude}`
        )
          .then((res) => res.json())
          .then((data: { features?: GeoJSONFeature[] }) => {
            const placeName = data.features?.[0]?.properties?.full_address
            if (placeName) {
              setSelectedLocation(placeName)
            }
          })
          .catch((err) => {
            console.error("Failed to reverse geocode geolocated position", err)
          })
      })

      // Only auto-center on the user's real location if permission was
      // already granted in a previous visit — never surprise a first-time
      // visitor with a location prompt before they've done anything. If
      // permission is still undecided or denied, the geolocate control
      // above remains available for the user to trigger explicitly.
      // Some browsers (and this app's own automated preview environment)
      // throw synchronously for unsupported permission queries rather
      // than rejecting the returned promise, so this whole check is
      // wrapped defensively.
      try {
        navigator.permissions
          ?.query({ name: "geolocation" })
          .then((status) => {
            if (status.state === "granted") {
              geolocate.trigger()
            }
          })
          .catch(() => {
            // Permission query unsupported/rejected — fall back to
            // requiring an explicit button click.
          })
      } catch {
        // Permissions API not available at all in this browser.
      }
    }

    // When a click event occurs on a feature in the places layer, open a popup at the
    // location of the feature, with description HTML from its properties.

    mapRef.current?.addInteraction("map-click", {
      type: "click",
      target: { layerId: "events" },
      handler: () => {
        if (selectedFeature) {
          mapRef.current?.setFeatureState(selectedFeature, { selected: false })
          setSelectedFeature(null)
        }
      },
    })
    // Change the cursor to a pointer when the mouse is over a POI.
    mapRef.current?.addInteraction("places-mouseenter-interaction", {
      type: "mouseenter",
      target: { layerId: "events" },
      handler: () => {
        if (mapRef.current) {
          mapRef.current.getCanvas().style.cursor = "pointer"
        }
      },
    })

    // Change the cursor back to a pointer when it stops hovering over a POI.
    mapRef.current?.addInteraction("places-mouseleave-interaction", {
      type: "mouseleave",
      target: { layerId: "events" },
      handler: () => {
        if (mapRef.current) {
          mapRef.current.getCanvas().style.cursor = ""
        }
      },
    })
    // Intentionally run once: this creates the Mapbox instance itself.
    // Re-running on every dependency change would tear down and recreate
    // the map, not just update it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const { latitude, longitude, start, end } = {
    latitude: selectedCoordinates[1],
    longitude: selectedCoordinates[0],
    ...dateRangeToApiParams(dateRange.start, dateRange.end),
  }
  useEffect(() => {
    if (!rendered) return
    void streamEvents({
      latitude,
      longitude,
      start,
      end,
    })

    return () => {
      cancelStream()
    }
  }, [latitude, longitude, start, end, streamEvents, cancelStream, rendered])

  useEffect(() => {
    const markEvents = () => {
      selectEvents([])

      const dataSource: FeatureCollection = {
        type: "FeatureCollection",
        features: [],
      }
      Object.values(eventsByDate).forEach((events: EventResponse[]) => {
        events.forEach((event) => {
          const { venue } = event

          if (!venue || !venue.location) {
            return
          }
          const location = venue.location

          const [longitude, latitude] = [
            parseFloat(location?.longitude ?? ""),
            parseFloat(location?.latitude ?? ""),
          ]
          const feature: Feature = {
            type: "Feature",
            properties: {
              description: event.name,
              venue: venue.name,
              color: "#373630",
              id: event.id,
              selected: "false",
            },
            geometry: {
              type: "Point",
              coordinates: [longitude, latitude],
            },
          }
          dataSource.features.push(feature)
        })
      })

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
            "icon-image": [
              "case",
              [
                "in",
                ["get", "id"],
                ["literal", selectedEvents.map((e) => e.id)],
              ],
              "marker-yellow",
              [
                "in",
                ["get", "clusterVenue"],
                ["literal", selectedEvents.map((e) => e.venue?.name)],
              ],

              "marker-yellow",
              "marker-red",
            ],
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
    // infinite loop. The effect above already patches icon-image via
    // setLayoutProperty when the selection changes, so rebuilding the whole
    // layer/source here on every click isn't needed either.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventsByDate, selectEvents])

  useEffect(() => {
    if (!mapRef.current) return

    mapRef.current.addInteraction("event-click-interaction", {
      type: "click",
      target: { layerId: "events" },
      handler: ({ feature }: InteractionEvent) => {
        const event: EventResponse | undefined = [
          ...Object.values(eventsByDate),
        ]
          .flat()
          ?.find((ev) => ev.id === feature?.id)
        const eventSource = mapRef.current?.getSource(
          "event-data-source"
        ) as GeoJSONSource
        if (event) {
          selectEvents([event])
        }
        eventSource.getClusterChildren(
          feature?.properties.cluster_id as number,
          (error, features) => {
            if (!error) {
              const eventIds = features?.map(
                (feature) => feature?.properties?.id
              )
              const selectedEvents = Object.values(eventsByDate).reduce(
                (acc, curr) => {
                  const selectedEvents = [...curr].filter((i) =>
                    eventIds?.includes(i.id)
                  )
                  acc.push(...selectedEvents)
                  return acc
                },
                []
              )

              selectEvents(selectedEvents)
            }
          }
        )
      },
    })

    return () => {
      mapRef.current?.removeInteraction("event-click-interaction")
    }
  }, [eventsByDate, selectEvents])

  useEffect(() => {
    if (!rendered) return
    mapRef.current?.easeTo({
      center: { lat: selectedCoordinates[1], lng: selectedCoordinates[0] },
      zoom: 12,
      speed: 0.8,
    })
  }, [selectedCoordinates, rendered])

  // Once a search had to widen past the base radius to find anything,
  // zoom out to frame the actual area covered instead of staying zoomed
  // in on a radius that came up empty.
  useEffect(() => {
    if (isStreaming || !radiusExpanded || !searchRadius) return
    mapRef.current?.fitBounds(
      boundsForRadius(selectedCoordinates, searchRadius),
      {
        padding: 60,
        duration: 800,
      }
    )
  }, [isStreaming, radiusExpanded, searchRadius, selectedCoordinates])

  return (
    <>
      <div className="relative flex min-h-0 flex-1">
        {!useIsMobile() && <DrawerWrapper />}
        <div ref={mapContainer} id="map-container" className="h-full w-full" />
      </div>
    </>
  )
}

export { MapWrapper }
