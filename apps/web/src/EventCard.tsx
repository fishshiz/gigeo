import type { EventGroup } from "./lib/groupEvents"
import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
import { HouseIcon } from "lucide-react"
import { useEventsContext } from "./providers/eventsProvider"
import { ResponsiveImage } from "@workspace/ui/components/ui/ResponsiveImage"
import { ForYouTag } from "./ForYouTag"
import { formatDate, formatTime } from "./lib/dates"

import { type ReactElement } from "react"
import type { EventResponse } from "./hooks/eventsStream"
const EventCard = ({
  event,
  date,
  onSelect,
}: {
  event: EventResponse
  date: ReactElement
  /** Overrides the default "select just this event" click behavior — used
   * by GroupedEventCard to select the whole group of showtimes instead. */
  onSelect?: () => void
}) => {
  const { selectEvents } = useEventsContext()
  const handleSelect = onSelect ?? (() => selectEvents([event]))

  return (
    <div
      role="button"
      tabIndex={0}
      className="relative flex cursor-pointer overflow-hidden rounded-xl border border-slate-100 bg-white shadow-sm transition hover:border-slate-200 hover:shadow-md active:scale-[0.99] dark:border-(--color-surface-dark-200) dark:bg-(--color-surface-dark-400) dark:hover:border-(--color-surface-dark-300)"
      onClick={handleSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault()
          handleSelect()
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
      <div className="flex h-auto min-w-0 flex-3 flex-col justify-between gap-2 p-3">
        <div className="flex flex-col items-start gap-1">
          <h3 className="leading-tight font-semibold text-(--text-primary)">
            {event.name}
          </h3>
          {event.matchedArtist && (
            <ForYouTag
              matchedArtist={event.matchedArtist}
              matchedVia={event.matchedVia}
            />
          )}
        </div>
        <div className="flex flex-col gap-1">
          <div>{date}</div>
          <div className="flex items-center gap-1 text-sm leading-none text-indigo-600 dark:text-(--text-secondary)">
            <HouseIcon aria-hidden className="h-4 w-4 shrink-0" />
            <span className="truncate">{event.venue?.name}</span>
          </div>
        </div>
      </div>
    </div>
  )
}

const GroupedEventCard = ({ group }: { group: EventGroup }) => {
  const { selectEvents } = useEventsContext()
  const primary = group.events[0]
  const final = group.events[group.events.length - 1]
  const count = group.events.length

  const dateFormatter = new DateFormatter("en-US", {
    month: "short",
    day: "numeric",
    timeZone: getLocalTimeZone(),
  })
  // Events without a specific start time come through as a bare "YYYY-MM-DD"
  // (no "T"), which parseAbsolute rejects as invalid.
  const dateLabel = primary.dates
    ? primary.dates.includes("T")
      ? dateFormatter.format(parseAbsolute(primary.dates, "UTC").toDate())
      : formatDate(primary.dates)
    : null

  return (
    <EventCard
      event={primary}
      date={
        <>
          {primary.dates && final.dates && (
            <span className="text-xs text-[#7C7A7A]">
              {dateLabel ? `${dateLabel} · ` : ""}
              {count} showtimes from {formatTime(primary.dates)} to{" "}
              {formatTime(final.dates)}
            </span>
          )}
        </>
      }
      onSelect={() => selectEvents(group.events)}
    />
  )
}

export { EventCard, GroupedEventCard }
