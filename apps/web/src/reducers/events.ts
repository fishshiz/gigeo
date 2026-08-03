import { type EventResponse, type EventsByDate } from "../hooks/eventsStream"

type EventsAction =
  | { type: "RESET_EVENTS" }
  | { type: "UPSERT_STREAMED_EVENT"; payload: EventResponse }
  | { type: "STREAM_ERROR"; payload: string }
  | { type: "STREAM_STATUS"; payload: { isStreaming: boolean } }
  | { type: "SELECT_EVENTS"; payload: EventResponse[] }

type EventsState = {
  eventsByDate: EventsByDate
  isStreaming: boolean
  selectedEvents: EventResponse[]
  error: string | null
}

function eventDateKey(event: EventResponse): string {
  if (!event.dates) return "unknown"

  // A bare "YYYY-MM-DD" (no time-of-day -- see normalize_event's
  // localDate-only fallback) already *is* the calendar day, with no
  // timezone to resolve. Returning it as-is avoids routing it through
  // `Date`, which would anchor it to UTC midnight and could read back a
  // day early in any timezone behind UTC (see below).
  if (/^\d{4}-\d{2}-\d{2}$/.test(event.dates)) return event.dates

  const date = new Date(event.dates)
  if (Number.isNaN(date.getTime())) return "unknown"

  // Use local, not UTC, calendar-day components. `event.dates` is often a
  // genuine UTC instant (e.g. "...T03:00:00Z"), and a late-evening show
  // converts to the *next* UTC calendar day -- an 11pm Eastern show is
  // 3am UTC the next day. Bucketing by the UTC day pushed those shows
  // into tomorrow's list instead of tonight's. This assumes the viewer's
  // timezone approximates the venue's, same as the rest of the app
  // (lib/dates.ts formats event times via `getLocalTimeZone()`).
  const year = date.getFullYear()
  const month = String(date.getMonth() + 1).padStart(2, "0")
  const day = String(date.getDate()).padStart(2, "0")
  return `${year}-${month}-${day}`
}

function sameEvent(a: EventResponse, b: EventResponse): boolean {
  if (a.id === b.id) return true

  const aVenue = a.venue?.name ?? ""
  const bVenue = b.venue?.name ?? ""
  const aPerformer = a.performers?.[0]?.id ?? ""
  const bPerformer = b.performers?.[0]?.id ?? ""

  return (
    (a.dates ?? "") === (b.dates ?? "") &&
    aVenue === bVenue &&
    aPerformer === bPerformer
  )
}

function sortEvents(events: EventResponse[]): EventResponse[] {
  return [...events].sort((a, b) => {
    const aTime = a.dates ? Date.parse(a.dates) : Number.MAX_SAFE_INTEGER
    const bTime = b.dates ? Date.parse(b.dates) : Number.MAX_SAFE_INTEGER

    if (aTime !== bTime) return aTime - bTime
    return a.name.localeCompare(b.name)
  })
}

function eventsReducer(state: EventsState, action: EventsAction): EventsState {
  switch (action.type) {
    case "RESET_EVENTS":
      return {
        eventsByDate: {},
        isStreaming: false,
        selectedEvents: [],
        error: null,
      }

    case "STREAM_STATUS":
      return {
        ...state,
        isStreaming: action.payload.isStreaming,
      }

    case "STREAM_ERROR":
      return {
        ...state,
        isStreaming: false,
        error: action.payload,
      }

    case "UPSERT_STREAMED_EVENT": {
      const event = action.payload
      const dateKey = eventDateKey(event)
      const existingBucket = state.eventsByDate[dateKey] ?? []

      const existingIndex = existingBucket.findIndex((existing) =>
        sameEvent(existing, event)
      )

      // A later emission for the same event (by sameEvent's identity) always
      // carries at least as much information as an earlier one — e.g. the
      // cross-source reconciliation pass re-emitting a Ticketmaster event
      // with rank/predictedAttendance attached — so replace rather than
      // discard on a match.
      const nextBucket =
        existingIndex === -1
          ? sortEvents([...existingBucket, event])
          : sortEvents(
              existingBucket.map((existing, i) =>
                i === existingIndex ? event : existing
              )
            )

      return {
        ...state,
        eventsByDate: {
          ...state.eventsByDate,
          [dateKey]: nextBucket,
        },
      }
    }

    case "SELECT_EVENTS": {
      const events = action.payload
      return {
        ...state,
        selectedEvents: events,
      }
    }

    default:
      return state
  }
}

const initialEventsState: EventsState = {
  eventsByDate: {},
  isStreaming: false,
  selectedEvents: [],
  error: null,
}

export {
  type EventsState,
  eventsReducer,
  initialEventsState,
  eventDateKey,
  sameEvent,
  sortEvents,
}
