import { type EventResponse } from "./hooks/eventsStream"
import { EventCard, GroupedEventCard } from "./EventCard"
import { Button } from "@workspace/ui/components/ui/Button"
import { ArrowLeftIcon } from "lucide-react"
import { useEventsContext } from "./providers/eventsProvider"
import { groupEvents } from "./lib/groupEvents"
import { formatDateTime } from "./lib/dates"
const VenueDetails = ({ events }: { events: EventResponse[] }) => {
  const { selectEvents } = useEventsContext()
  const venue = events[0].venue
  const groups = groupEvents(events)
  return (
    <div className="p-3">
      {/* bg-(--surface-secondary) is repeated under dark: because Button's own
          "secondary" variant carries its own dark:bg-neutral-700 -- without the
          explicit dark: pair, that wins the cascade over the unprefixed override
          (see the same pattern in EventDetails.tsx, confirmed empirically). */}
      <Button
        aria-label="Back to events"
        className="z-10 bg-(--surface-secondary) dark:border-(--color-border-subtle-dark-200) dark:bg-(--surface-secondary)"
        variant="secondary"
        onClick={() => selectEvents([])}
      >
        <ArrowLeftIcon aria-hidden className="h-4 w-4" />
      </Button>
      <h2 className="mt-3 leading-none font-semibold text-black dark:text-(--color-primary-dark-900)">
        {groups.length} Events at {venue?.name}
      </h2>
      <ul className="flex w-full flex-col justify-between gap-2.5 pt-4">
        {groups.map((group) => {
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
                      {primary.dates ? formatDateTime(primary.dates) : null}
                    </span>
                  }
                />
              )}
            </li>
          )
        })}
      </ul>
    </div>
  )
}

export { VenueDetails }
