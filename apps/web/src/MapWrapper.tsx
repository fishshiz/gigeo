import { useRef, useEffect, useState } from "react"
import mapboxgl, { GeoJSONSource, InteractionEvent } from "mapbox-gl"
import { useEvents } from "./components/events-provider"
import { useEventsContext } from "./providers/eventsProvider"
import { DrawerWrapper } from "./Drawer"
import { useIsMobile } from "./providers/Breakpoint"
import { DrawerTrigger } from "@workspace/ui/components/ui/Drawer"
import { ChevronRightIcon } from "lucide-react"
import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"
import type { Feature, FeatureCollection } from "geojson"
import type { EventResponse } from "./hooks/eventsStream"

const INITIAL_ZOOM = 1

const MapWrapper = () => {
  const [drawerOpen, setDrawerOpen] = useState(false)
  const eventsContext = useEvents()
  const [rendered, setRendered] = useState(false)
  useEffect(() => {
    setRendered(true)
  }, [])
  const { selectedCoordinates, setSelectedCoordinates, dateRange } =
    eventsContext

  const {
    streamEvents,
    cancelStream,
    eventsByDate,
    selectEvents,
    selectedEvents,
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

  const resizeObserver = new ResizeObserver(() => {
    mapRef.current?.resize()
  })
  if (mapContainer.current) {
    resizeObserver.observe(mapContainer.current)
  }

  useEffect(() => {
    if (
      selectedEvents.length &&
      selectedEvents[0]?.venue?.location !== undefined
    ) {
      const { latitude, longitude } = selectedEvents[0]?.venue.location
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

    if (!mapRef.current.hasControl(geolocate)) {
      // Add the control to the map.
      mapRef.current.addControl(geolocate, "bottom-right")
      // Set an event listener that fires
      // when a geolocate event occurs.
      geolocate.on("geolocate", (e) => {
        setSelectedCoordinates([e.coords.longitude, e.coords.latitude])
      })
    }
    mapRef.current?.addInteraction("event-click-interaction", {
      type: "click",
      target: { layerId: "events" },
      handler: (e) => {
        const event = [...Object.values(eventsByDate)]
          .flat()
          ?.find((ev) => ev.id === e?.feature?.id)
        if (event) {
          selectEvents([event])
        }
      },
    })

    // When a click event occurs on a feature in the places layer, open a popup at the
    // location of the feature, with description HTML from its properties.

    mapRef.current?.addInteraction("map-click", {
      type: "click",
      target: { layerId: "events" },
      handler: (e) => {
        const event = [...Object.values(eventsByDate)]
          .flat()
          ?.find((ev) => ev.id === e?.feature?.id)
        console.log(selectedFeature, e, event)
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
  }, [])

  const { latitude, longitude, radius, start, end } = {
    latitude: selectedCoordinates[1],
    longitude: selectedCoordinates[0],
    radius: 10,
    start: dateRange.start.toString() + "T00:00:00Z",
    end: dateRange.end.toString() + "T23:59:59Z",
  }
  useEffect(() => {
    if (!rendered) return
    void streamEvents({
      latitude,
      longitude,
      radius,
      start,
      end,
    })
    console.log("streaming", latitude, longitude, radius, start, end)

    return () => {
      cancelStream()
    }
  }, [latitude, longitude, radius, start, end, streamEvents, cancelStream])

  const markEvents = async () => {
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
        //   coordinates.push([longitude, latitude]);
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

      if (mapRef.current?.getLayer("events")) {
        mapRef.current.removeLayer("events")
      }
      if (mapRef.current?.getSource("event-data-source")) {
        mapRef.current.removeSource("event-data-source")
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
            ["in", ["get", "id"], ["literal", selectedEvents.map((e) => e.id)]],
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
    })
  }
  mapRef.current?.removeInteraction("event-click-interaction")
  mapRef.current?.addInteraction("event-click-interaction", {
    type: "click",
    target: { layerId: "events" },
    handler: ({ feature }: InteractionEvent) => {
      const event: EventResponse | undefined = [...Object.values(eventsByDate)]
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
            const eventIds = features?.map((feature) => feature?.properties?.id)
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

  useEffect(() => {
    markEvents()
  }, [eventsByDate])

  useEffect(() => {
    if (!rendered) return
    mapRef.current?.easeTo({
      center: { lat: selectedCoordinates[1], lng: selectedCoordinates[0] },
      zoom: 12,
      speed: 0.8,
    })
  }, [selectedCoordinates])

  mapRef.current?.on("load", () => {
    mapRef.current?.resize()
  })

  return (
    <>
      <div className="relative flex h-full min-h-0 flex-1">
        {!useIsMobile() && (
          <DrawerWrapper
            drawerOpen={drawerOpen}
            setDrawerOpen={(isOpen) => {
              setDrawerOpen(isOpen)
            }}
          />
        )}
        {!drawerOpen && (
          <DrawerTrigger
            onClick={() => setDrawerOpen(true)}
            className="shadow:lg absolute bottom-0 z-20 flex h-32 w-full items-center justify-center border-t border-gray-300 bg-white p-0 p-4 text-sm font-medium text-gray-700 sm:top-1/2 sm:left-0 sm:h-min sm:w-min sm:-translate-y-1/2 sm:transform sm:rounded-tr-lg sm:rounded-br-lg"
          >
            <ChevronRightIcon className="stroke-gray-700" />
          </DrawerTrigger>
        )}
        <div ref={mapContainer} id="map-container" className="h-full w-full" />
      </div>
    </>
  )
}

export { MapWrapper }
