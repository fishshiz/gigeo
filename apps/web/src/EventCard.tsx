import type { GroupedEvents } from "./lib/types"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
import { HouseIcon } from "lucide-react"
import { useEventsContext } from "./providers/eventsProvider"
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
  const { selectEvents } = useEventsContext()

  return (
    <div
      role="button"
      tabIndex={0}
      className="relative flex cursor-pointer overflow-hidden rounded-xl border border-slate-100 bg-white shadow-sm transition hover:border-slate-200 hover:shadow-md active:scale-[0.99] dark:border-(--color-surface-dark-200) dark:bg-(--color-surface-dark-400) dark:hover:border-(--color-surface-dark-300)"
      onClick={() => selectEvents([event])}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault()
          selectEvents([event])
        }
      }}
    >
      <div className="photo-detail h-auto w-full flex-1 overflow-hidden">
        <ResponsiveImage
          sources={event.images}
          style={{
            objectFit: "cover",
            objectPosition: "center",
            height: "100%",
            width: "100%",
          }}
          alt={event.name}
        />
      </div>
      <div className="flex h-auto flex-3 flex-col justify-between gap-2 p-3">
        <h3 className="leading-tight font-semibold text-black dark:text-(--color-primary-dark-900)">
          {event.name}
        </h3>
        <div className="flex flex-col gap-1">
          <div>{date}</div>
          <div className="flex items-center gap-1 text-sm leading-none text-indigo-600 dark:text-(--color-secondary-dark-900)">
            <HouseIcon aria-hidden className="h-4 w-4 shrink-0" />
            <span className="truncate">{event.venue?.name}</span>
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
