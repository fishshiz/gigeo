import { EventCard } from "../EventCard"
import { DateSlider } from "../DateSlider"

import { useEventsContext } from "../providers/eventsProvider"
import { useRef, type Ref } from "react"
import { useTopMostVisibleInScrollContainer } from "../hooks/listItemObserver"
import { formatDate } from "../lib/formats"
import { EventFilter } from "../EventFilter"

export const EventsDrawer = () => {
  const { eventsByDate, isStreaming, searchRadius, radiusExpanded } =
    useEventsContext()
  const eventListRef = useRef<HTMLDivElement>(null)
  const { registerItem } = useTopMostVisibleInScrollContainer(eventListRef, {
    offsetTop: 0,
  })
  const entries = Object.entries(eventsByDate).sort()

  if (!entries.length && isStreaming) {
    return (
      <div className="flex h-full w-full items-center justify-center">
        <p className="text-gray-600">Searching for events…</p>
      </div>
    )
  }

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
            {radiusExpanded && searchRadius
              ? `No events found within ${searchRadius}mi. Try searching for a different location, or adjusting the calendar date range.`
              : "No events found. Try searching for a different location, or adjusting the calendar date range."}
          </p>
        </div>
      )}
    </>
  )
}

export const EventsDrawerHeader = () => {
  const { eventsByDate, searchRadius, radiusExpanded } = useEventsContext()
  const eventListRef = useRef<HTMLDivElement>(null)

  const handleDateChange = (date: string) => {
    if (eventListRef && eventListRef.current) {
      const target = eventListRef.current.querySelector(`#a${date}`)

      if (target) {
        target.scrollIntoView({
          block: "start",
          behavior: "instant",
          // @ts-expect-error - `container` isn't in the standard ScrollIntoViewOptions type
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
      <div className="mb-2 flex w-full items-center justify-between">
        <h2 className="text-bold text-xl">Events</h2>
        {radiusExpanded && searchRadius && (
          <span className="text-xs text-gray-500">
            Showing events within {searchRadius}mi
          </span>
        )}
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

const DateAnchor = ({
  date,
  ref,
}: {
  date: string
  ref: Ref<HTMLDivElement>
}) => {
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
