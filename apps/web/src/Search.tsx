import { useState, useEffect, useRef } from "react"
import { useEvents } from "./components/events-provider"
import type { GeoJSONFeature } from "mapbox-gl"
import { TextField } from "@workspace/ui/components/ui/TextField"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"

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
  const { setSelectedCoordinates, dateRange, setDateRange } = useEvents()
  const [searchTerm, setSearchTerm] = useState("")
  const [searchedTerm, setSearchedTerm] = useState("")
  const [places, setPlaces] = useState<GeoJSONFeature[]>([])
  const listRef = useRef<HTMLUListElement | null>(null)

  const debounceValue = useDebounce(searchTerm, 500)

  useEffect(() => {
    fetch(`/api/cities?q=${debounceValue}`)
      .then((resp) => resp.json())
      .then((r) => setPlaces(r.features))
  }, [debounceValue])

  const updateSearchTerm = (e: string) => {
    if (searchedTerm.length) {
      setSearchedTerm("")
    }
    setSearchTerm(e)
  }
  const listboxId = "typeahead-listbox"

  const selectPlace = (place: GeoJSONFeature) => {
    console.log(place)
    if (place.geometry.type === "GeometryCollection") return
    const coordinates = place.geometry.coordinates as [number, number]
    setPlaces([place])
    setSelectedCoordinates([coordinates[0], coordinates[1]])
    setSearchedTerm(place.properties?.full_address)
  }

  return (
    <div className="relative w-full">
      <div className="absolute left-[50%] z-10 grid w-full translate-x-[-50%] grid-cols-1 grid-rows-[auto_1fr] p-4 sm:top-10 sm:w-md">
        <div className="relative flex h-fit max-h-[80px] w-full items-center overflow-hidden rounded-xl border-1 border-gray-300 bg-white">
          <TextField
            id="search"
            type="text"
            autoComplete="off"
            aria-label="Search for a city"
            name="search"
            value={searchedTerm ? searchedTerm : searchTerm}
            placeholder="Search for a city"
            className="block grow border-r-1 border-gray-300 p-0 text-base text-gray-900 outline-none placeholder:text-gray-400 focus:outline-none [&>input]:border-none"
            onChange={(e) => updateSearchTerm(e)}
          />
          <DateRangePicker
            aria-label="Select timeframe"
            value={dateRange}
            onChange={(date) => date && setDateRange(date)}
            className="[&>div]:border-none"
          />
        </div>
        {places.length > 1 && (
          <ul
            ref={listRef}
            id={listboxId}
            role="listbox"
            className="z-20 mt-1 max-h-60 w-full overflow-auto rounded-md border border-gray-200 bg-white py-1 text-sm shadow-lg"
          >
            {places.map((option, index) => (
              <li
                key={option.id}
                id={`typeahead-option-${index}`}
                role="option"
                tabIndex={-1}
                className={`flex w-full cursor-pointer items-center px-3 py-1.5`}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => selectPlace(option)}
              >
                {option.properties?.full_address}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  )
}

export { Search }
