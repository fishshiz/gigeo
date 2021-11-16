import { inject, InjectionKey, Ref } from "vue";
import { GeocodeFeature } from "../interface";

export const searchTermKey: InjectionKey<Ref<string>> = Symbol("searchTerm");
export const updateSearchTermKey: InjectionKey<Function> =
  Symbol("updateSearchTerm");
export const spotifyTokenKey: InjectionKey<Ref<string>> =
  Symbol("spotifyToken");

export const dateRangeKey: InjectionKey<Ref<[Date, Date]>> =
  Symbol("dateRange");
export const updateDateRangeKey: InjectionKey<Function> =
  Symbol("updateDateRange");
export const activeGeocodeKey: InjectionKey<Ref<GeocodeFeature>> =
  Symbol("activeGeocode");
export const updateActiveGeocodeKey: InjectionKey<Function> = Symbol(
  "updateActiveGeocode"
);
export function injectStrict<T>(key: InjectionKey<T>, fallback?: T) {
  const resolved = inject(key, fallback);
  if (!resolved) {
    throw new Error(`Could not resolve ${key.description}`);
  }
  return resolved;
}
