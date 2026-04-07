import type { GroupedEvents } from "./lib/types"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
import { HouseIcon } from "lucide-react"
import { useEvents } from "./components/events-provider"
import { ResponsiveImage } from "@workspace/ui/components/ui/ResponsiveImage"

import { type ReactElement } from "react"
import type { EventResponse } from "./hooks/eventsStream"
const EventCard = ({
  event,
  date,
}: {
  event: EventResponse
  date: ReactElement
}) => {
  const eventsContext = useEvents()

  return (
    <div
      className="shadow-8 flex overflow-hidden rounded-xl border border-slate-100 bg-white dark:border-(--color-surface-dark-200) dark:bg-(--color-surface-dark-400)"
      onClick={() => eventsContext.setSelectedEvent(event)}
    >
      <div className="photo-detail h-auto w-full flex-1 overflow-hidden">
        <ResponsiveImage
          sources={event.images as any}
          style={{
            objectFit: "cover",
            objectPosition: "center",
            height: "100%",
            width: "100%",
          }}
          alt="test"
        />
      </div>
      <div className="flex h-auto flex-3 flex-col justify-between p-2">
        <div>
          <h3 className="leading-none font-semibold text-black dark:text-(--color-primary-dark-900)">
            {event.name}
          </h3>
        </div>
        <div className="">
          <div>{date}</div>
          <div className="flex items-center text-base/7 leading-none text-indigo-600 dark:text-(--color-secondary-dark-900)">
            <HouseIcon aria-hidden className="mr-1 h-4 w-4" />
            <span className="">{event.venue?.name}</span>
          </div>
        </div>
      </div>
    </div>
  )
}

const GroupedEventCard = ({ events }: { events: GroupedEvents }) => {
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
