import { useRef, useEffect } from "react"
import mapboxgl from "mapbox-gl"
import { useEvents } from "./components/events-provider"

import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"
import type { Feature, FeatureCollection } from "geojson"
import type { Event, TmEvent } from "./lib/types"
import { formatDate } from "./lib/formats"

const INITIAL_ZOOM = 12.12

const MapWrapper = () => {
  const eventsContext = useEvents()
  const { selectedCoordinates, dateRange } = eventsContext
  const mapRef = useRef<mapboxgl.Map | null>(null)

  useEffect(() => {
    if (eventsContext.selectedEvent?.venue.location !== undefined) {
      const { latitude, longitude } =
        eventsContext.selectedEvent?.venue.location

      mapRef.current?.flyTo({
        center: { lat: parseFloat(latitude), lng: parseFloat(longitude) },
        speed: 0.8,
      })
    }
  }, [eventsContext.selectedEvent])

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
  }, [])

  const queryEvents = async () => {
    const events = await fetch(
      `/api/concerts?latitude=${selectedCoordinates[1]}&longitude=${selectedCoordinates[0]}&radius=10&start=${dateRange.start}T00:00:00Z&end=${dateRange.end}T23:59:59Z`
    ).then((r) => r.json())
    const eventsToUpdate: Record<string, Event[]> = events.reduce(
      (acc: Record<string, Event[]>, curr: TmEvent) => {
        const date = formatDate(curr.dates)
        const obj = { datesPretty: date, ...curr }
        if (!acc[date]) {
          acc[date] = [obj]
        } else {
          acc[date].push(obj)
        }
        return acc
      },
      {}
    )
    eventsContext.setEvents(eventsToUpdate)

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
          ["boolean", ["has", "icon"], false],
          ["get", "icon"],
          "bullseye-solid",
        ],
        "icon-size": 0.6,
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
