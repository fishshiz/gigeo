import { useRef, useEffect, useState } from "react"
import mapboxgl, { InteractionEvent } from "mapbox-gl"
import { useEvents } from "./components/events-provider"
import { useEventsContext } from "./providers/eventsProvider"

import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"
import type { Feature, FeatureCollection } from "geojson"
import type { EventResponse } from "./hooks/eventsStream"

const INITIAL_ZOOM = 12.12

const MapWrapper = ({ drawerOpen }: { drawerOpen: boolean }) => {
  const eventsContext = useEvents()
  const {
    selectedCoordinates,
    setSelectedCoordinates,
    setSelectedEvent,
    selectedEvent,
    events,
    dateRange,
  } = eventsContext
  const mapRef = useRef<mapboxgl.Map | null>(null)
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
    setTimeout(() => {
      console.log("FIRED", mapRef.current)
      mapRef.current?.resize()
    }, 10)
  }, [drawerOpen])

  useEffect(() => {
    if (selectedEvent?.venue?.location !== undefined) {
      const { latitude, longitude } = selectedEvent?.venue.location
      mapRef.current?.setLayoutProperty("events", "icon-image", [
        "match",
        ["get", "id"],
        selectedEvent.id,
        "marker-yellow", //image when id is the hovered feature id
        "marker-red", // default
      ])
      if (latitude && longitude) {
        mapRef.current?.flyTo({
          center: { lat: parseFloat(latitude), lng: parseFloat(longitude) },
          speed: 0.8,
        })
      }
    }
  }, [selectedEvent])

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
      console.log("dsf")
      mapRef.current.addControl(geolocate)
    }
    // Set an event listener that fires
    // when a geolocate event occurs.
    geolocate.on("geolocate", (e) => {
      console.log("A geolocate event has occurred.", e)
      setSelectedCoordinates([e.coords.longitude, e.coords.latitude])
    })
    mapRef.current?.addInteraction("event-click-interaction", {
      type: "click",
      target: { layerId: "events" },
      handler: (e) => {
        const event = [...Object.values(events)]
          .flat()
          ?.find((ev) => ev.id === e?.feature?.id)
        console.log("click", event, e)
        setSelectedEvent(event)
      },
    })

    // When a click event occurs on a feature in the places layer, open a popup at the
    // location of the feature, with description HTML from its properties.

    mapRef.current?.addInteraction("map-click", {
      type: "click",
      handler: (e) => {
        const event = [...Object.values(events)]
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

  const { streamEvents, cancelStream, eventsByDate } = useEventsContext()
  const { latitude, longitude, radius, start, end } = {
    latitude: selectedCoordinates[1],
    longitude: selectedCoordinates[0],
    radius: 10,
    start: dateRange.start.toString() + "T00:00:00Z",
    end: dateRange.end.toString() + "T23:59:59Z",
  }
  useEffect(() => {
    void streamEvents({
      latitude,
      longitude,
      radius,
      start,
      end,
    })

    return () => {
      cancelStream()
    }
  }, [latitude, longitude, radius, start, end, streamEvents, cancelStream])

  const markEvents = async () => {
    setSelectedEvent(undefined)
    console.log("ahsahas", eventsByDate)

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
          "icon-image": "marker-red",
          "icon-size": 1.6,
        },
        paint: {
          "icon-color": "#ea4236",
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
  console.log("Event clicked:")
  if (mapRef.current?.removeInteraction("event-click-interaction")) {
    console.log("Removed existing event-click-interaction")
  }
  mapRef.current?.addInteraction("event-click-interaction", {
    type: "click",
    target: { layerId: "events" },
    handler: ({ feature }: InteractionEvent) => {
      const event: EventResponse | undefined = [...Object.values(eventsByDate)]
        .flat()
        ?.find((ev) => ev.id === feature?.id)
      console.log("clickity,", [...Object.values(events)], feature)
      if (event && feature) {
        mapRef.current?.setLayoutProperty("events", "icon-image", [
          "match",
          ["get", "id"],
          feature.id,
          "marker-yellow", //image when id is the hovered feature id
          "marker-red", // default
        ])
        setSelectedEvent(event)
      }
    },
  })

  useEffect(() => {
    markEvents()
  }, [eventsByDate])

  useEffect(() => {
    mapRef.current?.easeTo({
      center: { lat: selectedCoordinates[1], lng: selectedCoordinates[0] },
      speed: 0.8,
    })
  }, [selectedCoordinates])

  mapRef.current?.on("load", () => {
    mapRef.current?.resize()
  })

  return (
    <div
      id="map-container"
      className="h-full w-full"
      style={{ height: "100%", width: "100%" }}
    />
  )
}

export { MapWrapper }
