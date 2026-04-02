import { useRef, useEffect, useState } from "react"
import mapboxgl, { InteractionEvent } from "mapbox-gl"
import { useEvents } from "./components/events-provider"

import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"
import type { Feature, FeatureCollection } from "geojson"
import type { Event, TmEvent } from "./lib/types"

const INITIAL_ZOOM = 12.12
const isMobile = window.innerWidth < 768

const MapWrapper = () => {
  const eventsContext = useEvents()
  const {
    selectedCoordinates,
    setSelectedEvent,
    selectedEvent,
    setEvents,
    events,
    dateRange,
  } = eventsContext
  const mapRef = useRef<mapboxgl.Map | null>(null)
  const [selectedFeature, setSelectedFeature] = useState(null)

  useEffect(() => {
    if (selectedEvent?.venue.location !== undefined) {
      const { latitude, longitude } = selectedEvent?.venue.location
      mapRef.current?.setLayoutProperty("events", "icon-image", [
        "match",
        ["get", "id"],
        selectedEvent.id,
        "marker-yellow", //image when id is the hovered feature id
        "marker-red", // default
      ])
      const drawerHeight = window.innerHeight * 0.5
      const drawerWidth = window.innerWidth * 0.3
      mapRef.current?.flyTo({
        center: { lat: parseFloat(latitude), lng: parseFloat(longitude) },
        padding: {
          top: 0,
          right: 0,
          bottom: isMobile ? drawerHeight : 0,
          left: !isMobile ? drawerWidth : 0,
        }, // Adjust padding to account for the drawer
        speed: 0.8,
      })
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
    // mapRef.current?.addInteraction("event-click-interaction", {
    //   type: "click",
    //   target: { layerId: "events" },
    //   handler: (e) => {
    //     const event = [...Object.values(events)]
    //       .flat()
    //       ?.find((ev) => ev.id === e?.feature?.id)

    //     setSelectedEvent(event)
    //   },
    // })

    // When a click event occurs on a feature in the places layer, open a popup at the
    // location of the feature, with description HTML from its properties.

    mapRef.current?.addInteraction("map-click", {
      type: "click",
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
  }, [])

  const queryEvents = async () => {
    const events = await fetch(
      `/api/concerts?latitude=${selectedCoordinates[1]}&longitude=${selectedCoordinates[0]}&radius=10&start=${dateRange.start}T00:00:00Z&end=${dateRange.end}T23:59:59Z`
    ).then((r) => r.json())
    const eventsToUpdate: Record<string, Event[]> = events.reduce(
      (acc: Record<string, Event[]>, curr: TmEvent) => {
        const obj = { ...curr }
        if (!acc[curr.datesPretty]) {
          acc[curr.datesPretty] = [obj]
        } else {
          acc[curr.datesPretty].push(obj)
        }
        return acc
      },
      {}
    )
    setSelectedEvent(undefined)
    setEvents(eventsToUpdate)
    mapRef.current?.removeInteraction("event-click-interaction")

    const dataSource: FeatureCollection = {
      type: "FeatureCollection",
      features: [],
    }
    events.forEach((place: Event) => {
      const { venue } = place

      if (!venue.location) {
        return
      }
      const location = venue.location

      const [longitude, latitude] = [
        parseFloat(location.longitude),
        parseFloat(location.latitude),
      ]
      //   coordinates.push([longitude, latitude]);
      const feature: Feature = {
        type: "Feature",
        properties: {
          description: place.name,
          venue: venue.name,
          color: "#373630",
          id: place.id,
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
  }
  console.log("Event clicked:")
  if (mapRef.current?.removeInteraction("event-click-interaction")) {
    console.log("Removed existing event-click-interaction")
  }
  mapRef.current?.addInteraction("event-click-interaction", {
    type: "click",
    target: { layerId: "events" },
    handler: ({ feature }: InteractionEvent) => {
      const event: Event | undefined = [...Object.values(events)]
        .flat()
        ?.find((ev) => ev.id === feature?.id)
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
    mapRef.current?.easeTo({
      center: { lat: selectedCoordinates[1], lng: selectedCoordinates[0] },
      speed: 0.8,
    })
    queryEvents()
  }, [selectedCoordinates, dateRange])

  return <div id="map-container" style={{ height: "100vh", width: "100%" }} />
}

export { MapWrapper }
