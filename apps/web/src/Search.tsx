import { useState, useEffect, useRef } from "react"
import type { GeoJSONFeature } from "mapbox-gl"
import { TextField } from "@workspace/ui/components/ui/TextField"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"
import { useSearchProvider } from "./providers/searchProvider"
import { useDrawerProvider } from "./providers/drawerProvider"
import { useNavigateToLocation } from "./hooks/useNavigateToLocation"
import { locationFromFeature } from "./lib/mapbox"

const useDebounce = (value: string, delayTime: number) => {
  const [debounceValue, setDebounceValue] = useState(value)

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebounceValue(value)
    }, delayTime)
    return () => {
      clearTimeout(timer)
    }
  }, [value, delayTime])
  return debounceValue
}

const Search = () => {
  const {
    dateRange,
    setDateRange,
    selectedLocation,
    setSelectedLocation,
    setInputRef,
  } = useSearchProvider()
  const { setIsDrawerOpen } = useDrawerProvider()
  const navigateToLocation = useNavigateToLocation()

  const [searchTerm, setSearchTerm] = useState("")
  const [places, setPlaces] = useState<GeoJSONFeature[]>([])
  const [isSuggestionsOpen, setIsSuggestionsOpen] = useState(false)
  const [activeIndex, setActiveIndex] = useState<number | null>(null)
  const listRef = useRef<HTMLUListElement | null>(null)
  const inputRef = useRef<HTMLInputElement | null>(null)

  const debounceValue = useDebounce(searchTerm, 500)
  const isOpen = isSuggestionsOpen && places.length > 1
  const listboxId = "typeahead-listbox"
  const activeOptionId =
    activeIndex !== null ? `typeahead-option-${activeIndex}` : undefined

  useEffect(() => {
    setInputRef(inputRef.current)
    // TextField's props type doesn't recognize `role`, so the combobox role
    // (there's no dedicated `<input type="combobox">`) is set imperatively.
    inputRef.current?.setAttribute("role", "combobox")
  }, [setInputRef])

  // react-aria-components' TextField/useTextField owns `aria-expanded` and
  // `aria-activedescendant` internally and drops them if passed as JSX
  // props (unlike `aria-controls`/`aria-autocomplete`, which pass through
  // fine) — so they're kept in sync on the DOM node directly instead.
  useEffect(() => {
    const el = inputRef.current
    if (!el) return
    el.setAttribute("aria-expanded", String(isOpen))
    if (isOpen && activeOptionId) {
      el.setAttribute("aria-activedescendant", activeOptionId)
    } else {
      el.removeAttribute("aria-activedescendant")
    }
  }, [isOpen, activeOptionId])

  useEffect(() => {
    fetch(`/api/cities?q=${debounceValue}`)
      .then((resp) => resp.json())
      .then((r) => {
        setPlaces(r.features)
        setActiveIndex(null)
        setIsSuggestionsOpen(true)
      })
  }, [debounceValue])

  const updateSearchTerm = (e: string) => {
    if (selectedLocation?.fullAddress.length) {
      setSelectedLocation(undefined)
    }
    setSearchTerm(e)
  }

  const selectPlace = (place: GeoJSONFeature) => {
    if (place.geometry.type === "GeometryCollection") return
    const coordinates = place.geometry.coordinates as [number, number]
    const location = locationFromFeature(place)
    setPlaces([place])
    setIsSuggestionsOpen(false)
    setActiveIndex(null)
    navigateToLocation([coordinates[0], coordinates[1]], location)
    setIsDrawerOpen(true)
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!isOpen) return
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault()
        setActiveIndex((i) => (i === null ? 0 : (i + 1) % places.length))
        break
      case "ArrowUp":
        e.preventDefault()
        setActiveIndex((i) =>
          i === null
            ? places.length - 1
            : (i - 1 + places.length) % places.length
        )
        break
      case "Enter":
        if (activeIndex !== null) {
          e.preventDefault()
          selectPlace(places[activeIndex])
        }
        break
      case "Escape":
        e.preventDefault()
        setIsSuggestionsOpen(false)
        setActiveIndex(null)
        break
    }
  }

  return (
    <div className="relative w-full p-4 sm:max-w-lg">
      <div className="relative flex h-fit max-h-[80px] w-full items-center overflow-hidden rounded-xl border-1 border-gray-300 bg-white">
        {/* TextField doesn't forward arbitrary input attributes like
            `title` down to the underlying <input> -- wrapping it in a
            `contents` span (removed from the box model entirely, so it
            can't affect the flex layout) is the least invasive way to
            make the full, possibly-truncated value discoverable on
            hover/long-press without touching the shared component. */}
        <span
          title={selectedLocation ? selectedLocation.fullAddress : searchTerm}
          className="contents"
        >
          <TextField
            id="search"
            ref={inputRef}
            type="text"
            autoComplete="off"
            aria-label="Search for a city"
            aria-autocomplete="list"
            aria-controls={listboxId}
            name="search"
            value={selectedLocation ? selectedLocation.fullAddress : searchTerm}
            placeholder="Search for a city"
            className="block w-full grow border-r-1 border-gray-300 p-0 outline-none focus:outline-none [&>input]:border-none"
            inputClassName="truncate text-base text-gray-900 placeholder:text-gray-400"
            onChange={(e) => updateSearchTerm(e)}
            onKeyDown={handleKeyDown}
            onFocus={() => places.length > 1 && setIsSuggestionsOpen(true)}
            onBlur={() => setIsSuggestionsOpen(false)}
          />
        </span>
        <DateRangePicker
          aria-label="Select timeframe"
          value={dateRange}
          onChange={(date) => date && setDateRange(date)}
          className="[&>div]:border-none"
        />
      </div>
      {isOpen && (
        <ul
          ref={listRef}
          id={listboxId}
          role="listbox"
          className="absolute z-20 mt-1 max-h-60 w-full overflow-auto rounded-md border border-gray-200 bg-white py-1 text-sm shadow-lg"
        >
          {places.map((option, index) => (
            <li
              key={option.id}
              id={`typeahead-option-${index}`}
              role="option"
              aria-selected={index === activeIndex}
              tabIndex={-1}
              className={`flex w-full cursor-pointer items-center px-3 py-1.5 ${
                index === activeIndex ? "bg-gray-100" : ""
              }`}
              onMouseDown={(e) => e.preventDefault()}
              onMouseEnter={() => setActiveIndex(index)}
              onClick={() => selectPlace(option)}
            >
              {option.properties?.full_address}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export { Search }
