import type { Event } from "./lib/types"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"

import { useState } from "react"
const EventCard = ({ event }: { event: Event }) => {
  let [src, setSrc] = useState(event.images.find((i) => i.ratio === "4_3")?.url)
  const date = parseAbsolute(event.dates, "UTC")
  const dateFormatter = new DateFormatter("en-US", {
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone: getLocalTimeZone(), // or a specific IANA tz
  })
  const dateString = dateFormatter.format(date.toDate())
  console.log(date, dateFormatter.format(date.toDate()))
  return (
    <div className="bg-white-p10 ring1 relative mb-2 flex rounded-3xl shadow-2xl ring-gray-900/10">
      <div className="photo-detail">
        <img src={src} className="h-[75px] w-[100px]" />
      </div>
      <div>
        <h3 className="text-base/7 font-semibold text-indigo-600">
          {event.name}
        </h3>
        <span>{dateString}</span>
        <span>{event.venue.name}</span>
      </div>
    </div>
  )
}

export { EventCard }
