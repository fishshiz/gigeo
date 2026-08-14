import { useState, useEffect, useRef } from "react"
import type { GeoJSONFeature } from "mapbox-gl"
import { motion, useReducedMotion } from "motion/react"
import { MOTION_DURATION, MOTION_EASE } from "@workspace/ui/lib/motion"
import { TextField } from "@workspace/ui/components/ui/TextField"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"
import { useSearchProvider } from "./providers/searchProvider"
import { useDrawerProvider } from "./providers/drawerProvider"
import { useIsMobile } from "./providers/Breakpoint"
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
  const isMobile = useIsMobile()
  const shouldReduceMotion = useReducedMotion()

  const [searchTerm, setSearchTerm] = useState("")
  const [places, setPlaces] = useState<GeoJSONFeature[]>([])
  const [isSuggestionsOpen, setIsSuggestionsOpen] = useState(false)
  const [activeIndex, setActiveIndex] = useState<number | null>(null)
  const listRef = useRef<HTMLUListElement | null>(null)
  const inputRef = useRef<HTMLInputElement | null>(null)

  // Mobile-only: the two fields stay segmented (both always visible and
  // interactive -- never hidden behind a summary), but each one's own
  // share of the row's width shifts based on which is active, so the one
  // being used gets breathing room instead of the two permanently fighting
  // over a fixed split. Desktop keeps the original static layout -- no
  // width pressure to solve there. isDatePickerOpen is tracked separately
  // from the text field's own focus since they're independent controls.
  const [isTextFocused, setIsTextFocused] = useState(false)
  const [isDatePickerOpen, setIsDatePickerOpen] = useState(false)

  const locationWrapperClass = !isMobile
    ? "flex-auto grow min-w-0"
    : isTextFocused
      ? "flex-[3] min-w-0"
      : "flex-[1.3] min-w-0"
  const dateWrapperClass = !isMobile
    ? "flex-none"
    : isDatePickerOpen
      ? "flex-[1.6] min-w-[112px]"
      : "flex-1 min-w-[90px]"

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

  // TextField (packages/ui) spreads its own ...props onto the *outer*
  // AriaTextField wrapper, not onto the underlying <input> -- which is
  // where `ref` (and so inputRef here) actually points. An onFocus/onBlur
  // JSX prop on TextField itself doesn't reliably reach the native input's
  // own focus/blur, so this listens on the real DOM node directly instead
  // (confirmed to be the right node: it's the same ref used elsewhere in
  // this file to call .focus() imperatively).
  useEffect(() => {
    const el = inputRef.current
    if (!el) return
    const onFocus = () => {
      setIsTextFocused(true)
      // Selects whatever's already there (a previously-picked address, or
      // leftover typed text) so tapping back in and typing immediately
      // replaces it -- no manual clear needed to search a new location.
      el.select()
    }
    const onBlur = () => setIsTextFocused(false)
    el.addEventListener("focus", onFocus)
    el.addEventListener("blur", onBlur)
    return () => {
      el.removeEventListener("focus", onFocus)
      el.removeEventListener("blur", onBlur)
    }
  }, [])

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
    <>
      <div className="relative w-full p-4 sm:max-w-lg">
        <div className="relative flex h-fit max-h-[80px] w-full items-center overflow-hidden rounded-xl border-1 border-gray-300 bg-white">
          {/* Each field is its own layout-animated flex item -- rather
              than the whole bar collapsing to a single summary, both
              fields stay segmented and always interactive, and just trade
              width with each other based on which one is active. */}
          <motion.div
            layout
            transition={
              shouldReduceMotion
                ? { duration: 0 }
                : { duration: MOTION_DURATION.base, ease: MOTION_EASE.out }
            }
            className={locationWrapperClass}
          >
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
                value={
                  selectedLocation ? selectedLocation.fullAddress : searchTerm
                }
                placeholder="Search for a city"
                className="block w-full grow border-r-1 border-gray-300 p-0 outline-none focus:outline-none [&>input]:border-none"
                inputClassName="truncate text-base text-gray-900 placeholder:text-gray-400"
                onChange={(e) => updateSearchTerm(e)}
                onKeyDown={handleKeyDown}
                onFocus={() => {
                  if (places.length > 1) setIsSuggestionsOpen(true)
                }}
                onBlur={() => setIsSuggestionsOpen(false)}
              />
            </span>
          </motion.div>
          <motion.div
            layout
            transition={
              shouldReduceMotion
                ? { duration: 0 }
                : { duration: MOTION_DURATION.base, ease: MOTION_EASE.out }
            }
            className={dateWrapperClass}
          >
            <DateRangePicker
              aria-label="Select timeframe"
              value={dateRange}
              onChange={(date) => date && setDateRange(date)}
              onOpenChange={setIsDatePickerOpen}
              className="w-full [&>div]:border-none"
            />
          </motion.div>
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
    </>
  )
}

export { Search }
