import type { EventResponse } from "../hooks/eventsStream"
import { eventDateKey, sortEvents } from "../reducers/events"

type EventGroup = {
  events: EventResponse[]
}

function eventGroupKey(event: EventResponse): string {
  return `${event.name}__${event.venue?.name ?? ""}__${eventDateKey(event)}`
}

function eventTime(event: EventResponse): number {
  return event.dates ? Date.parse(event.dates) : Number.MAX_SAFE_INTEGER
}

/** Collapses same-day, same-venue showtimes of the same event (e.g. a
 * museum exhibit running hourly) into single groups, ordered by their
 * earliest showtime. */
function groupEvents(events: EventResponse[]): EventGroup[] {
  const buckets = new Map<string, EventResponse[]>()

  for (const event of events) {
    const key = eventGroupKey(event)
    const bucket = buckets.get(key)
    if (bucket) {
      bucket.push(event)
    } else {
      buckets.set(key, [event])
    }
  }

  return Array.from(buckets.values())
    .map((bucketEvents) => ({ events: sortEvents(bucketEvents) }))
    .sort((a, b) => {
      const aTime = eventTime(a.events[0])
      const bTime = eventTime(b.events[0])
      if (aTime !== bTime) return aTime - bTime
      return a.events[0].name.localeCompare(b.events[0].name)
    })
}

export { groupEvents, type EventGroup }
