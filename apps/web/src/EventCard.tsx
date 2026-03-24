import type { Event } from "./lib/types"

import { useState } from "react"
const EventCard = ({ event }: { event: Event }) => {
  let [src, setSrc] = useState(event.images[0].url)

  return (
    <div className="bg-white-p10 ring1 relative rounded-3xl shadow-2xl ring-gray-900/10">
      <div className="photo-detail">
        <img
          src={src}
          style={
            {
              "--width": event.images[0].width,
              "--height": event.images[0].height,
            } as any
          }
        />
      </div>
      <h3 className="text-base/7 font-semibold text-indigo-600">
        {event.name}
      </h3>

      <span>{event.venue.name}</span>
    </div>
  )
}

export { EventCard }
