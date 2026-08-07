import { type EventResponse } from "./hooks/eventsStream"
import { EventCard, GroupedEventCard } from "./EventCard"
import { Button } from "@workspace/ui/components/ui/Button"
import { ArrowLeftIcon, MapPinIcon, ExternalLinkIcon } from "lucide-react"
import { useEventsContext } from "./providers/eventsProvider"
import { groupEvents } from "./lib/groupEvents"
import { formatDateTime } from "./lib/dates"

/** "123 Main St, Austin, TX 78712" -- standard US street/city/state/zip
 * order, built from whichever of the four parts Ticketmaster actually
 * sent (any can be absent, see `TmVenue` on the backend). */
function formatVenueAddress(venue: EventResponse["venue"]): string | null {
  if (!venue) return null
  const cityState = [venue.city, venue.stateCode].filter(Boolean).join(", ")
  const cityStateZip = [cityState, venue.postalCode].filter(Boolean).join(" ")
  const line = [venue.address, cityStateZip].filter(Boolean).join(", ")
  return line || null
}

const VenueDetails = ({ events }: { events: EventResponse[] }) => {
  const { selectEvents } = useEventsContext()
  const venue = events[0].venue
  const groups = groupEvents(events)
  const address = formatVenueAddress(venue)
  const venueImage = venue?.images?.[0]

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
      <div className="mt-3 flex items-start gap-3">
        {venueImage && (
          <img
            src={venueImage.url}
            alt=""
            className="h-12 w-12 shrink-0 rounded object-cover"
          />
        )}
        <div className="min-w-0">
          <h2 className="leading-none font-semibold text-(--text-primary)">
            {groups.length} Events at {venue?.name}
          </h2>
          {address && (
            <p className="mt-1.5 flex items-start gap-1 text-sm text-muted-foreground">
              <MapPinIcon aria-hidden className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              <span>{address}</span>
            </p>
          )}
          {venue?.url && (
            <a
              href={venue.url}
              target="_blank"
              rel="noreferrer"
              className="mt-1 inline-flex items-center gap-1 text-sm text-(--text-link) hover:underline"
            >
              View on Ticketmaster
              <ExternalLinkIcon aria-hidden className="h-3 w-3 shrink-0" />
            </a>
          )}
        </div>
      </div>
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
