import type { Event, GroupedEvents } from "./lib/types"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
import { HouseIcon } from "lucide-react"
import { useEvents } from "./components/events-provider"

import { useState, type ReactElement } from "react"
const EventCard = ({ event, date }: { event: Event; date: ReactElement }) => {
  let [src, setSrc] = useState(event.images.find((i) => i.ratio === "4_3")?.url)
  const eventsContext = useEvents()

  return (
    <div
      className="shadow-8 flex overflow-hidden rounded-xl border border-slate-100 bg-white dark:bg-[#494949]"
      onClick={() => eventsContext.setSelectedEvent(event)}
    >
      <div className="photo-detail max-w-[160px]">
        <img src={src} className="h-auto w-full" />
      </div>
      <div className="p-2">
        <h3 className="font-semibold text-black">{event.name}</h3>
        <div>{date}</div>
        <div className="flex items-center text-base/7 font-semibold text-indigo-600">
          <HouseIcon aria-hidden className="h-4 w-4" />
          <span>{event.venue.name}</span>
        </div>
      </div>
    </div>
  )
}

const GroupedEventCard = ({ events }: { events: GroupedEvents }) => {
  let [src, setSrc] = useState(
    events.events[0].images.find((i) => i.ratio === "4_3")?.url
  )
  const startDate = parseAbsolute(events.dateRange.start, "UTC")
  const endDate = parseAbsolute(events.dateRange.end, "UTC")
  const dateFormatter = new DateFormatter("en-US", {
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone: getLocalTimeZone(), // or a specific IANA tz
  })
  const dateString = `${dateFormatter.format(startDate.toDate())} - ${dateFormatter.format(endDate.toDate())}`
  return (
    <>
      <span className="absolute top-[-8px] right-[-8px] m-auto flex h-9 w-12 items-center justify-center rounded-4xl bg-red-600 p-3 text-center text-xs font-thin outline outline-rose-500 dark:bg-[#FF5D73]">
        {events.events.length} events
      </span>
      <EventCard
        event={events.events[0]}
        date={<span className="text-xs text-[#7C7A7A]">{dateString}</span>}
      />
    </>
  )
}

export { EventCard, GroupedEventCard }
