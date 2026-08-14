import { EventCard, GroupedEventCard } from "../EventCard"
import { DateSlider } from "../DateSlider"

import { useEventsContext } from "../providers/eventsProvider"
import { useSearchProvider } from "@/providers/searchProvider"
import { useMemo } from "react"
import type { Ref } from "react"
import { formatDate, formatDateTime } from "../lib/dates"
import { EventFilter, EventFilterChips } from "../EventFilter"
import { groupEvents } from "../lib/groupEvents"

type RegisterItem = (id: string) => Ref<HTMLDivElement>

export const EventsDrawer = ({
  registerItem,
}: {
  registerItem: RegisterItem
}) => {
  const {
    eventsByDate,
    visibleEventsByDate,
    isStreaming,
    searchRadius,
    radiusExpanded,
    activeClassifications,
    forYouOnly,
  } = useEventsContext()
  const entries = Object.entries(visibleEventsByDate).sort()
  const hasActiveFilters = activeClassifications.size > 0 || forYouOnly
  const filteredEverythingOut =
    hasActiveFilters && !entries.length && Object.keys(eventsByDate).length > 0

  if (!entries.length && isStreaming) {
    return (
      <div className="flex h-full w-full items-center justify-center p-6 text-center">
        <p className="text-sm text-gray-600 dark:text-gray-400">
          Searching for events…
        </p>
      </div>
    )
  }

  return (
    <div className="px-4">
      {entries.length ? (
        <div className="flex flex-col gap-4 pb-4">
          {entries.map(([date, events]) => (
            <div key={date}>
              <DateAnchor date={date} ref={registerItem(date)} />
              <ul className="flex w-full flex-col justify-between gap-2.5 pt-2.5">
                {groupEvents(events).map((group) => {
                  const primary = group.events[0]
                  return (
                    <li className="relative" key={primary.id}>
                      {group.events.length > 1 ? (
                        <GroupedEventCard group={group} />
                      ) : (
                        <EventCard
                          event={primary}
                          date={
                            <span className="text-gray-600 dark:text-gray-400">
                              {primary.dates
                                ? formatDateTime(primary.dates)
                                : null}
                            </span>
                          }
                        />
                      )}
                    </li>
                  )
                })}
              </ul>
            </div>
          ))}
        </div>
      ) : (
        <div className="flex h-full w-full items-center justify-center p-6 text-center">
          <p className="text-sm text-gray-600 dark:text-gray-400">
            {filteredEverythingOut
              ? "No events match the selected filters. Try clearing a filter."
              : radiusExpanded && searchRadius
                ? `No events found within ${searchRadius}mi. Try searching for a different location, or adjusting the calendar date range.`
                : "No events found. Try searching for a different location, or adjusting the calendar date range."}
          </p>
        </div>
      )}
    </div>
  )
}

export const EventsDrawerHeader = ({
  topMostId,
  onSelect,
}: {
  topMostId: string | null
  onSelect: (date: string) => void
}) => {
  const { visibleEventsByDate, searchRadius, radiusExpanded } =
    useEventsContext()
  const { selectedLocation, dateRange } = useSearchProvider()
  const entries = Object.entries(visibleEventsByDate).sort()
  const eventDates = useMemo(
    () => new Set(entries.map(([key]) => key)),
    [entries]
  )

  return (
    <>
      <div className="mb-2 flex w-full items-start justify-between md:pr-10">
        <div className="flex flex-col gap-1">
          <h2 className="text-left text-xl font-bold">
            {selectedLocation?.cityName
              ? `Events in ${selectedLocation.cityName}`
              : "Events"}
          </h2>
          {radiusExpanded && searchRadius && (
            <span className="text-left text-xs text-gray-500">
              Showing events within {searchRadius}mi
            </span>
          )}
        </div>
        <EventFilter />
      </div>
      <EventFilterChips />
      <DateSlider
        dateRange={dateRange}
        eventDates={eventDates}
        activeDateId={topMostId}
        onSelect={onSelect}
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
    <div className="relative -mx-4">
      <div id={`a${date}`} />
      <div
        ref={ref}
        className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) px-3 py-1.5 dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700)"
      >
        <h3 className="text-sm font-semibold text-white dark:text-(--color-text-primary-dark-700)">
          {formattedDate}
        </h3>
      </div>
    </div>
  )
}
