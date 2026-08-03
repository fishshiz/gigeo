import type { GeoJSONFeature } from "mapbox-gl"
import type { Location } from "./types"

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
