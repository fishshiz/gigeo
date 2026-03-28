import { EventCard, GroupedEventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { useState } from "react"
import { EventDetails } from "./EventDetails"
import { Search } from "./Search"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"
import type { Event, GroupedEvents } from "./lib/types"
import { formatDate } from "./lib/formats"
import { ChevronRight } from "lucide-react"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
import { Link } from "@workspace/ui/components/ui/Link"
const Drawer = () => {
  const eventsContext = useEvents()
  const {
    events,
    dateRange,
    setDateRange,
    selectedCoordinates,
    setSelectedCoordinates,
  } = eventsContext

  const [drawerOpen, setDrawerOpen] = useState(false)
  // const normalizedEvents: (Event | GroupedEvents)[] = Object.values(
  //   Object.entries(events).forEach(entry).reduce((acc: Record<string, Event | GroupedEvents>, curr: Event) => {
  //     const primaryAttraction = curr.attractions
  //       ? curr.attractions[0].name
  //       : curr.name
  //     const key = `${primaryAttraction}_${curr.venue.name}`
  //     if (!acc[key]) {
  //       acc[key] = curr
  //     } else if ("grouped" in acc[key]) {
  //       const oldEvent = acc[key]
  //       acc[key].events.push(curr)
  //       acc[key].dateRange.start =
  //         parseAbsolute(curr.dates, "UTC").compare(
  //           parseAbsolute(oldEvent.dateRange.start, "UTC")
  //         ) < 0
  //           ? curr.dates
  //           : oldEvent.dateRange.start
  //       acc[key].dateRange.end =
  //         parseAbsolute(curr.dates, "UTC").compare(
  //           parseAbsolute(oldEvent.dateRange.end, "UTC")
  //         ) > 0
  //           ? curr.dates
  //           : oldEvent.dateRange.end
  //       acc[key].attractions = curr.attractions
  //     } else {
  //       const oldEvent = acc[key]
  //       acc[key] = {
  //         name: curr.name,
  //         grouped: true,
  //         venue: curr.venue.name,
  //         attractions: curr.attractions,
  //         dateRange: {
  //           start:
  //             parseAbsolute(curr.dates, "UTC").compare(
  //               parseAbsolute(oldEvent.dates, "UTC")
  //             ) < 0
  //               ? curr.dates
  //               : oldEvent.dates,
  //           end:
  //             parseAbsolute(curr.dates, "UTC").compare(
  //               parseAbsolute(oldEvent.dates, "UTC")
  //             ) > 0
  //               ? curr.dates
  //               : oldEvent.dates,
  //         },
  //         events: [oldEvent, curr],
  //       }
  //     }
  //     return acc
  //   }, {})
  // )
  return (
    <div className="z-10 flex h-full max-h-screen min-h-screen w-full basis-2xl flex-col bg-white shadow-xl sm:top-0 sm:w-md dark:bg-(--color-bg-dark-900)">
      <div className="relative top-0 z-10 flex border-b border-b-slate-200 bg-white px-1 py-2 dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-900)">
        <Search dispatchPlace={setSelectedCoordinates} />
        <DateRangePicker
          aria-label="Select timeframe"
          value={dateRange}
          onChange={(date) => setDateRange(date)}
        />
      </div>
      {eventsContext.selectedEvent ? (
        <EventDetails eventData={eventsContext.selectedEvent} />
      ) : (
        <EventList events={events} />
      )}
    </div>
  )
}

const EventList = ({ events }: { events: Record<string, Event[]> }) => {
  const entries = Object.entries(events)
  const formatDateId = (date: string) =>
    date.toLocaleLowerCase().replace(" ", "")
  return (
    <div className="overflow-y-scroll scroll-smooth">
      {entries.map(([date, events], idx) => (
        <div key={formatDateId(date)}>
          <div
            id={formatDateId(date)}
            className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) p-4 dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700) dark:text-(--color-text-primary-dark-700)"
          >
            <h3>{date}</h3>
            {idx < entries.length - 1 && (
              <Link href={`#${formatDateId(entries[idx + 1][0])}`}>
                <ChevronRight />
              </Link>
            )}
          </div>
          <ul className="flex w-full flex-col justify-between gap-4 p-4">
            {events.map((event) => (
              <li className="relative">
                <EventCard
                  key={event.id}
                  event={event}
                  date={
                    <span className="text-gray-600">
                      {formatDate(event.dates)}
                    </span>
                  }
                />
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  )
}

export { Drawer }
