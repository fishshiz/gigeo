import { type EventResponse, type EventsByDate } from "../hooks/eventsStream"

type EventsAction =
  | { type: "RESET_EVENTS" }
  | { type: "UPSERT_STREAMED_EVENT"; payload: EventResponse }
  | { type: "STREAM_ERROR"; payload: string }
  | { type: "STREAM_STATUS"; payload: { isStreaming: boolean } }

type EventsState = {
  eventsByDate: EventsByDate
  isStreaming: boolean
  error: string | null
}

function eventDateKey(event: EventResponse): string {
  if (event.datesPretty) {
    return event.datesPretty
  }
  console.log("else", event)
  if (event.dates) {
    const date = new Date(event.dates)
    if (!Number.isNaN(date.getTime())) {
      const formatted = date.toLocaleDateString("en-US", {
        month: "long",
        day: "2-digit",
      })
      console.log(formatted)
      return formatted
    }
  }

  return "unknown"
}

function sameEvent(a: EventResponse, b: EventResponse): boolean {
  if (a.id === b.id) return true

  const aVenue = a.venue?.name ?? ""
  const bVenue = b.venue?.name ?? ""
  const aAttraction = a.attractions?.[0]?.id ?? ""
  const bAttraction = b.attractions?.[0]?.id ?? ""

  return (
    (a.dates ?? "") === (b.dates ?? "") &&
    aVenue === bVenue &&
    aAttraction === bAttraction
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

      const alreadyExists = existingBucket.some((existing) =>
        sameEvent(existing, event)
      )

      if (alreadyExists) {
        return state
      }

      const nextBucket = sortEvents([...existingBucket, event])

      return {
        ...state,
        eventsByDate: {
          ...state.eventsByDate,
          [dateKey]: nextBucket,
        },
      }
    }

    default:
      return state
  }
}

const initialEventsState: EventsState = {
  eventsByDate: {},
  isStreaming: false,
  error: null,
}

export { type EventsState, eventsReducer, initialEventsState }
