import { EventCard, GroupedEventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { useState } from "react"
import { EventDetails } from "./EventDetails"
import { Search } from "./Search"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"
import type { Event, GroupedEvents } from "./lib/types"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
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
    <div className="absolute top-20 bottom-0 left-0 z-10 h-full max-h-screen w-full bg-white shadow-xl sm:top-0 sm:w-md dark:bg-black">
      <div className="relative top-0 z-10 flex border-b border-b-slate-200 bg-white px-1 py-2">
        <Search dispatchPlace={setSelectedCoordinates} />
        <DateRangePicker
          aria-label="Select timeframe"
          value={dateRange}
          onChange={(_e) => setDateRange}
        />
      </div>
      <div className="h-full">
        {eventsContext.selectedEvent ? (
          <EventDetails eventData={eventsContext.selectedEvent} />
        ) : (
          <EventList events={events} />
        )}
      </div>
    </div>
  )
}

const EventList = ({ events }: { events: Record<string, Event[]> }) => {
  return (
    <div className="h-full overflow-y-scroll">
      {Object.entries(events).map(([date, events]) => (
        <div>
          <div className="sticky top-0 z-10 border-b border-b-slate-200 bg-white p-4 text-violet-600">
            <h3>{date}</h3>
          </div>
          <ul className="flex w-full flex-col justify-between gap-4 p-4">
            {events.map((event) => (
              <li className="relative">
                <EventCard
                  key={event.id}
                  event={event}
                  date={
                    <span className="text-gray-600">
                      {new DateFormatter("en-US", {
                        month: "long",
                        day: "numeric",
                        hour: "numeric",
                        minute: "2-digit",
                        hour12: true,
                        timeZone: getLocalTimeZone(), // or a specific IANA tz
                      }).format(parseAbsolute(event.dates, "UTC").toDate())}
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
