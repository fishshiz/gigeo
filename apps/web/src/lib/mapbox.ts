import type { ExpressionSpecification, GeoJSONFeature } from "mapbox-gl"
import type { Feature, FeatureCollection } from "geojson"
import type { Location } from "./types"
import type { EventResponse, EventsByDate } from "../hooks/eventsStream"

/**
 * Parses a Mapbox Geocoding v6 `place` feature (from /api/cities or
 * /api/reverse-geocode, which proxy that API's response shape unchanged)
 * into the app's Location shape.
 */
export function locationFromFeature(
  feature: GeoJSONFeature
): Location | undefined {
  if (feature.geometry.type !== "Point") return undefined

  const properties = feature.properties
  const coordinates = feature.geometry.coordinates as [number, number]
  const context = properties?.context

  if (!properties?.full_address || !properties?.name) return undefined

  return {
    fullAddress: properties.full_address,
    cityName: properties.name,
    stateCode: context?.region?.region_code ?? "",
    countryCode: context?.country?.country_code ?? "",
    coordinates,
  }
}

// ---------------------------------------------------------------------------
// The "events" layer: a GeoJSON source built from `eventsByDate`, clustered
// by venue, rendered as markers/cluster labels. `useEventLayer` (the only
// caller) owns the imperative Mapbox side of this; the two functions below
// are its pure core -- the actual data-shaping, testable without Mapbox.
// ---------------------------------------------------------------------------

/** Builds the GeoJSON `FeatureCollection` backing the "events" source from
 * every currently-loaded event. Events with no venue location are skipped
 * silently -- they have nothing to plot. */
export function buildEventFeatureCollection(
  eventsByDate: EventsByDate
): FeatureCollection {
  const dataSource: FeatureCollection = {
    type: "FeatureCollection",
    features: [],
  }

  Object.values(eventsByDate).forEach((events) => {
    events.forEach((event) => {
      const { venue } = event
      if (!venue?.location) return

      const [longitude, latitude] = [
        parseFloat(venue.location.longitude ?? ""),
        parseFloat(venue.location.latitude ?? ""),
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

  return dataSource
}

/** The "events" layer's `icon-color` expression: `selectedColor` for the
 * selected event(s) or any marker sharing a selected venue (so a cluster
 * containing a selected show also highlights), `defaultColor` otherwise.
 *
 * Consolidates two expressions that had quietly drifted apart: layer
 * creation matched *any* selected id (`"in"`), while the reactive
 * selection-change update only ever matched `selectedEvents[0]`'s id
 * (`"=="`) -- so a multi-event selection (e.g. from clicking a cluster)
 * only highlighted one marker after the first update. This is the "in"
 * behavior, used everywhere now.
 *
 * Colors are passed in rather than hardcoded so this stays a pure,
 * DOM-free function -- see `resolveCssColor` for how the caller resolves
 * the app's actual accent tokens at runtime. */
export function eventIconColorExpression(
  selectedEvents: EventResponse[],
  defaultColor: string,
  selectedColor: string
): ExpressionSpecification {
  return [
    "case",
    ["in", ["get", "id"], ["literal", selectedEvents.map((e) => e.id)]],
    selectedColor,
    [
      "in",
      ["get", "clusterVenue"],
      ["literal", selectedEvents.map((e) => e.venue?.name)],
    ],
    selectedColor,
    defaultColor,
  ] as unknown as ExpressionSpecification
}

/** Resolves a CSS custom property (e.g. "--accent-bg") to a plain
 * `rgba()` string, honoring whichever of .light/.dark is currently on
 * <html> -- Mapbox GL expressions need a real color, and these tokens are
 * defined in OKLCH, which its color parser can't read directly.
 *
 * Not `getComputedStyle(el).color`: modern browsers increasingly preserve
 * the original color syntax there (returning the oklch() string back
 * unchanged) rather than always normalizing to rgb(), which is exactly
 * the syntax Mapbox can't parse. Painting onto a canvas and reading the
 * pixel back sidesteps that -- canvas `fillStyle` accepts any CSS color
 * syntax, and `getImageData` always returns concrete 0-255 sRGB bytes
 * regardless of how the color was originally specified. */
export function resolveCssColor(varName: string): string {
  const raw = getComputedStyle(document.documentElement)
    .getPropertyValue(varName)
    .trim()

  const canvas = document.createElement("canvas")
  canvas.width = 1
  canvas.height = 1
  const ctx = canvas.getContext("2d")
  if (!ctx) return raw

  ctx.fillStyle = raw
  ctx.fillRect(0, 0, 1, 1)
  const [r, g, b, a] = ctx.getImageData(0, 0, 1, 1).data
  return `rgba(${r}, ${g}, ${b}, ${a / 255})`
}

/** Resolves Mapbox feature ids (from a click or `getClusterChildren`) back
 * to the real `EventResponse`s they represent. */
export function resolveEventsByIds(
  eventsByDate: EventsByDate,
  ids: readonly unknown[]
): EventResponse[] {
  return Object.values(eventsByDate).reduce<EventResponse[]>((acc, events) => {
    acc.push(...events.filter((event) => ids.includes(event.id)))
    return acc
  }, [])
}
