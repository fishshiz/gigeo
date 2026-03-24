import { type RefObject } from "react"

import "mapbox-gl/dist/mapbox-gl.css"

const Map = ({ mapContainerRef }: { mapContainerRef: RefObject<any> }) => {
  return <div id="map-container" ref={mapContainerRef} />
}

export { Map }
