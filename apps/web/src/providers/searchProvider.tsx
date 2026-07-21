/* eslint-disable react-refresh/only-export-components */
import * as React from "react"
import {
  type CalendarDate,
  today,
  getLocalTimeZone,
} from "@internationalized/date"
import { type RangeValue } from "react-aria"

const INITIAL_CENTER: [number, number] = [-24, 42]

type SearchProviderState = {
  selectedLocation: string | undefined
  setSelectedLocation: (location: string | undefined) => void
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
    string | undefined
  >(undefined)
  const [selectedCoordinates, setSelectedCoordinates] =
    React.useState<[number, number]>(INITIAL_CENTER)
  const [dateRange, setDateRange] = React.useState<RangeValue<CalendarDate>>({
    start: today(getLocalTimeZone()),
    end: today(getLocalTimeZone()).add({ weeks: 1 }),
  })
  const inputRef = React.useRef<HTMLInputElement | null>(null)

  const focusSearchInput = () => {
    console.log("Focusing search input", inputRef.current)
    inputRef.current?.focus()
  }
  const setInputRef = (ref: HTMLInputElement | null) => {
    console.log("Setting input ref", ref)
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
