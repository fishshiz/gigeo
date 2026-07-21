import { EventCard } from "../EventCard"
import { EventDetails } from "../EventDetails"
import { XIcon } from "lucide-react"
import { DateSlider } from "../DateSlider"
import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"
import { useEventsContext } from "../providers/eventsProvider"
import { useMediaQuery } from "usehooks-ts"
import { useEffect, useRef, type Ref } from "react"
import { useTopMostVisibleInScrollContainer } from "../hooks/listItemObserver"
import { formatDate } from "../lib/formats"
import { VenueDetails } from "../VenueDetails"
import { EventFilter } from "../EventFilter"

export const EventsDrawer = () => {
  const { eventsByDate, selectedEvents } = useEventsContext()
  const eventListRef = useRef<HTMLDivElement>(null)
  const { topMostId, registerItem } = useTopMostVisibleInScrollContainer(
    eventListRef,
    {
      offsetTop: 0,
    }
  )
  const entries = Object.entries(eventsByDate).sort()
  return (
    <>
      {entries.length ? (
        <div ref={eventListRef}>
          {entries.map(([date, events]) => (
            <div key={date}>
              <DateAnchor date={date} ref={registerItem(date)} />
              <ul className="flex w-full flex-col justify-between gap-4">
                {events.map((event) => (
                  <li className="relative" key={event.id}>
                    <EventCard
                      key={event.id}
                      event={event}
                      date={
                        <span className="text-gray-600">{event.dates}</span>
                      }
                    />
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
      ) : (
        <div className="flex h-full w-full items-center justify-center">
          <p className="text-gray-600">
            No events found. Try searching for a different location, or
            adjusting the calendar date range.
          </p>
        </div>
      )}
    </>
  )
}

export const EventsDrawerHeader = () => {
  const { eventsByDate, selectedEvents } = useEventsContext()
  const eventListRef = useRef<HTMLDivElement>(null)

  const handleDateChange = (date: string) => {
    if (eventListRef && eventListRef.current) {
      const target = eventListRef.current.querySelector(`#a${date}`)

      if (target) {
        target.scrollIntoView({
          block: "start",
          behavior: "instant",
          // @ts-ignore
          container: "nearest",
        })
      }
    }
  }

  const { topMostId } = useTopMostVisibleInScrollContainer(eventListRef, {
    offsetTop: 0,
  })
  const entries = Object.entries(eventsByDate).sort()

  return (
    <>
      <div className="mb-2 flex w-full">
        <h2 className="text-bold text-xl">Events</h2>
      </div>
      <EventFilter />
      <DateSlider
        dates={entries.map(([key]) => key)}
        activeDateId={topMostId}
        onSelect={handleDateChange}
      />
    </>
  )
}

const DateAnchor = ({ date, ref }: { date: string; ref: Ref<any> }) => {
  const formattedDate = formatDate(date)
  return (
    <>
      <div id={`a${date}`} />
      <div
        ref={ref}
        className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700) dark:text-(--color-text-primary-dark-700)"
      >
        <h3>{formattedDate}</h3>
      </div>
    </>
  )
}
