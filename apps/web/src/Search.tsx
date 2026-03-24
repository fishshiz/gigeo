import { useState, useEffect, useRef } from "react"
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

const Search = ({ dispatchPlace }: { dispatchPlace: Function }) => {
  const [searchTerm, setSearchTerm] = useState("")
  const [searchedTerm, setSearchedTerm] = useState("")
  const [places, setPlaces] = useState<GeoJSONFeature[]>([])
  const [activeIndex, setActiveIndex] = useState<number | null>(null)
  const listRef = useRef<HTMLUListElement | null>(null)

  const debounceValue = useDebounce(searchTerm, 1500)

  useEffect(() => {
    fetch(`/api/cities?q=${debounceValue}`)
      .then((resp) => resp.json())
      .then((r) => setPlaces(r.features))
  }, [debounceValue])

  const updateSearchTerm = (e: string) => {
    console.log("changed input")
    if (searchedTerm.length) {
      setSearchedTerm("")
    }
    setSearchTerm(e)
  }
  const listboxId = "typeahead-listbox"

  const selectPlace = (place: GeoJSONFeature) => {
    setPlaces([place])
    dispatchPlace(place)
    setSearchedTerm(place.properties.full_address)
  }

  return (
    <div>
      <TextField
        id="search"
        type="text"
        name="search"
        value={searchedTerm ? searchedTerm : searchTerm}
        placeholder="Search for ..."
        className="block min-w-0 grow pr-3 pl-1 text-base text-gray-900 placeholder:text-gray-400 focus:outline-none sm:text-sm/6"
        onChange={(e) => updateSearchTerm(e)}
      />
      {places.length > 1 && (
        <ul
          ref={listRef}
          id={listboxId}
          role="listbox"
          className="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-md border border-gray-200 bg-white py-1 text-sm shadow-lg"
        >
          {places.map((option, index) => (
            <li
              key={option.id}
              id={`typeahead-option-${index}`}
              role="option"
              tabIndex={-1}
              className={`flex cursor-pointer items-center px-3 py-1.5 ${
                index === activeIndex
                  ? "bg-indigo-600 text-white"
                  : "text-gray-900 hover:bg-gray-100"
              }`}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => selectPlace(option)}
            >
              {option.properties.full_address}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export { Search }
