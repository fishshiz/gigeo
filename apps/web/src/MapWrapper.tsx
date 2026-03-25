import { useRef, useEffect, useState } from "react"
import { Search } from "./Search"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"
import { Map } from "./Map"
import mapboxgl from "mapbox-gl"
import { ICONS } from "./constants"
import { useEvents } from "./components/events-provider"
import {
  today,
  getLocalTimeZone,
  type CalendarDate,
} from "@internationalized/date"
import { type RangeValue } from "react-aria"

import "mapbox-gl/dist/mapbox-gl.css"
import "./App.css"
import type { Feature, FeatureCollection } from "geojson"
import type { Event, TmEvent } from "./lib/types"
import { formatDateTime } from "./lib/formats"

const INITIAL_CENTER: [number, number] = [-74.0242, 40.6941]
const INITIAL_ZOOM = 12.12

const MapWrapper = () => {
  const eventsContext = useEvents()
  const mapRef = useRef<mapboxgl.Map | null>(null)
  const mapContainerRef = useRef<HTMLElement | string>("map-container")
  let [range, setRange] = useState<RangeValue<CalendarDate>>({
    start: today(getLocalTimeZone()),
    end: today(getLocalTimeZone()).add({ weeks: 1 }),
  })

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

  const [center, setCenter] = useState<[number, number]>(INITIAL_CENTER)
  const theme =
    window.matchMedia &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
      ? import.meta.env.VITE_MAPBOX_DARK_STYLE
      : import.meta.env.VITE_MAPBOX_LIGHT_STYLE
  useEffect(() => {
    mapboxgl.accessToken = import.meta.env.VITE_MAPBOX_ACCESS_TOKEN
    mapRef.current = new mapboxgl.Map({
      style: theme,
      container: mapContainerRef.current,
      center: center,
      zoom: INITIAL_ZOOM,
    })
    Promise.all(
      ICONS.map(
        (img) =>
          new Promise<void>((resolve, reject) => {
            mapRef.current?.loadImage(
              `./src/assets/${img}.png`,
              (error, res: any) => {
                mapRef.current?.addImage(img, res, { sdf: true })
                resolve()
              }
            )
          })
      )
    )
  }, [])

  const queryEvents = async () => {
    if (mapRef.current?.getLayer("events")) {
      mapRef.current.removeLayer("events")
    }
    if (mapRef.current?.getSource("event-data-source")) {
      mapRef.current.removeSource("event-data-source")
    }
    const events = await fetch(
      `/api/concerts?latitude=${center[1]}&longitude=${center[0]}&radius=10&start=${range.start}T00:00:00Z&end=${range.end}T23:59:59Z`
    ).then((r) => r.json())
    const eventsToUpdate: Event[] = [
      ...events.map((e: TmEvent) => ({
        datesPretty: formatDateTime(e.dates),
        ...e,
      })),
    ]
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
      center: { lat: center[1], lng: center[0] },
      speed: 0.8,
    })
    queryEvents()
  }, [center])

  const setCoordinates = (place: mapboxgl.GeoJSONFeature) => {
    const { coordinates } = place.properties
    console.log(coordinates)
    setCenter([coordinates.longitude, coordinates.latitude])
  }

  const setDateRange = (dateRange: RangeValue<CalendarDate>) => {
    setRange(dateRange)
    queryEvents()
  }

  return (
    <div className="absolute top-0 left-0 block h-full w-full">
      <div className="align-center absolute top-5 right-0 left-0 z-10 m-auto flex w-min justify-center p-2">
        <Search dispatchPlace={setCoordinates} />
        <DateRangePicker
          aria-label="Select timeframe"
          value={range}
          onChange={(e) => setDateRange(e)}
        />
      </div>
      <Map mapContainerRef={mapContainerRef} />
    </div>
  )
}

export { MapWrapper }
