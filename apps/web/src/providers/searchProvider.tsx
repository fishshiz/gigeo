/* eslint-disable react-refresh/only-export-components */
import * as React from "react"
import {
  type CalendarDate,
  today,
  getLocalTimeZone,
} from "@internationalized/date"
import { type RangeValue } from "react-aria"
import type { Location } from "../lib/types"

// Geographic center of the contiguous US — used only when there's no
// remembered location and geolocation permission hasn't already been
// granted. Previously this defaulted to the middle of the Atlantic.
const DEFAULT_CENTER: [number, number] = [-98.5795, 39.8283]

const COORDINATES_STORAGE_KEY = "gigeo:lastCoordinates"
const LOCATION_STORAGE_KEY = "gigeo:lastLocation"

function readStoredCoordinates(): [number, number] | undefined {
  try {
    const raw = localStorage.getItem(COORDINATES_STORAGE_KEY)
    if (!raw) return undefined
    const parsed = JSON.parse(raw)
    if (
      Array.isArray(parsed) &&
      parsed.length === 2 &&
      typeof parsed[0] === "number" &&
      typeof parsed[1] === "number"
    ) {
      return [parsed[0], parsed[1]]
    }
  } catch {
    // localStorage unavailable (e.g. private browsing) or malformed value —
    // fall through to the default.
  }
  return undefined
}

function readStoredLocation(): Location | undefined {
  try {
    const raw = localStorage.getItem(LOCATION_STORAGE_KEY)
    if (!raw) return undefined
    return JSON.parse(raw)
  } catch {
    return undefined
  }
}

type SearchProviderState = {
  selectedLocation: Location | undefined
  setSelectedLocation: (location: Location | undefined) => void
  selectedCoordinates: [number, number]
  setSelectedCoordinates: (coordinates: [number, number]) => void
  dateRange: RangeValue<CalendarDate>
  setDateRange: (date: RangeValue<CalendarDate>) => void
  focusSearchInput: () => void
  setInputRef: (ref: HTMLInputElement | null) => void
}

type SearchProviderProps = {
  children: React.ReactNode
}

export const SearchProviderContext = React.createContext<
  SearchProviderState | undefined
>(undefined)

export function SearchProvider({ children }: SearchProviderProps) {
  const [selectedLocation, setSelectedLocation] = React.useState<
    Location | undefined
  >(readStoredLocation)
  const [selectedCoordinates, setSelectedCoordinates] = React.useState<
    [number, number]
  >(() => readStoredCoordinates() ?? DEFAULT_CENTER)
  const [dateRange, setDateRange] = React.useState<RangeValue<CalendarDate>>({
    start: today(getLocalTimeZone()),
    end: today(getLocalTimeZone()).add({ weeks: 1 }),
  })
  const inputRef = React.useRef<HTMLInputElement | null>(null)

  // Remember the last selected location so a returning visitor starts
  // where they left off instead of the static default every time.
  React.useEffect(() => {
    try {
      localStorage.setItem(
        COORDINATES_STORAGE_KEY,
        JSON.stringify(selectedCoordinates)
      )
    } catch {
      // Ignore — localStorage may be unavailable (e.g. private browsing).
    }
  }, [selectedCoordinates])

  React.useEffect(() => {
    try {
      if (selectedLocation) {
        localStorage.setItem(
          LOCATION_STORAGE_KEY,
          JSON.stringify(selectedLocation)
        )
      } else {
        localStorage.removeItem(LOCATION_STORAGE_KEY)
      }
    } catch {
      // Ignore — localStorage may be unavailable (e.g. private browsing).
    }
  }, [selectedLocation])

  const focusSearchInput = () => {
    inputRef.current?.focus()
  }
  const setInputRef = (ref: HTMLInputElement | null) => {
    inputRef.current = ref
  }

  return (
    <SearchProviderContext
      value={{
        selectedLocation,
        setSelectedLocation,
        selectedCoordinates,
        setSelectedCoordinates,
        dateRange,
        setDateRange,
        focusSearchInput,
        setInputRef,
      }}
    >
      {children}
    </SearchProviderContext>
  )
}

export const useSearchProvider = () => {
  const context = React.useContext(SearchProviderContext)

  if (context === undefined) {
    throw new Error("useSearchProvider must be used within a SearchProvider")
  }

  return context
}
