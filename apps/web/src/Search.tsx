import { useState, useEffect, useRef } from "react"
import { useEvents } from "./components/events-provider"
import type { GeoJSONFeature } from "mapbox-gl"
import { TextField } from "@workspace/ui/components/ui/TextField"

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
  const { setSelectedCoordinates } = useEvents()
  const [searchTerm, setSearchTerm] = useState("")
  const [searchedTerm, setSearchedTerm] = useState("")
  const [places, setPlaces] = useState<GeoJSONFeature[]>([])
  const listRef = useRef<HTMLUListElement | null>(null)

  const debounceValue = useDebounce(searchTerm, 1500)

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
    <div className="flex-1">
      <TextField
        id="search"
        type="text"
        autoComplete="off"
        aria-label="Search for a city"
        name="search"
        value={searchedTerm ? searchedTerm : searchTerm}
        placeholder="Search for ..."
        className="block grow pr-3 pl-1 text-base text-gray-900 placeholder:text-gray-400 focus:outline-none sm:text-sm/6"
        onChange={(e) => updateSearchTerm(e)}
      />
      {places.length > 1 && (
        <ul
          ref={listRef}
          id={listboxId}
          role="listbox"
          className="absolute z-20 mt-1 max-h-60 w-[calc(100%-var(--spacing))] overflow-auto rounded-md border border-gray-200 bg-white py-1 text-sm shadow-lg"
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
  )
}

export { Search }
