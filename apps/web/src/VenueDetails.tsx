import { type EventResponse } from "./hooks/eventsStream"
import { EventCard } from "./EventCard"
import { Button } from "@workspace/ui/components/ui/Button"
import { ArrowLeftIcon } from "lucide-react"
import { useEventsContext } from "./providers/eventsProvider"
const VenueDetails = ({ events }: { events: EventResponse[] }) => {
  const { selectEvents } = useEventsContext()
  const venue = events[0].venue
  return (
    <div className="p-3">
      <Button
        aria-label="Back to events"
        className="z-10 dark:border-(--color-border-subtle-dark-200) dark:bg-(--color-dusty-olive-dark-600)"
        variant="secondary"
        onClick={() => selectEvents([])}
      >
        <ArrowLeftIcon aria-hidden className="h-4 w-4" />
      </Button>
      <h2 className="mt-3 leading-none font-semibold text-black dark:text-(--color-primary-dark-900)">
        {events.length} Events at {venue?.name}
      </h2>
      <ul className="flex w-full flex-col justify-between gap-2.5 pt-4">
        {events.map((event) => (
          <li className="relative" key={event.id}>
            <EventCard
              event={event}
              date={
                <span className="text-gray-600 dark:text-gray-400">
                  {event.dates}
                </span>
              }
            />
          </li>
        ))}
      </ul>
    </div>
  )
}

export { VenueDetails }
