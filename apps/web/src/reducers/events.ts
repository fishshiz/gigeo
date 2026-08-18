import { type EventResponse, type EventsByDate } from "../hooks/eventsStream"
import { eventDateKey } from "../lib/dates"

// Mirrors apps/api/src/events/reconcile.rs's normalize_venue_spelling --
// folds British/American spelling variants (confirmed live: Ticketmaster's
// "Bluebird Theatre" vs. PredictHQ's "Bluebird Theater" for the same Denver
// venue) before stripping punctuation, so this fallback can catch the same
// case as the backend's cross-source matcher.
const VENUE_SPELLING_ALIASES: [RegExp, string][] = [
  [/theatre/g, "theater"],
  [/centre/g, "center"],
]

const normalizeVenueName = (name: string): string => {
  const lower = name.toLowerCase()
  const aliased = VENUE_SPELLING_ALIASES.reduce(
    (acc, [variant, canonical]) => acc.replace(variant, canonical),
    lower
  )
  return aliased.replace(/[^a-z0-9]+/g, "")
}

const performerIdsOverlap = (a: EventResponse, b: EventResponse): boolean => {
  const aIds = new Set(
    (a.performers ?? []).flatMap((p) => (p.id ? [p.id] : []))
  )
  if (aIds.size === 0) return false
  return (b.performers ?? []).some((p) => p.id && aIds.has(p.id))
}

type EventsAction =
  | { type: "RESET_EVENTS" }
  | { type: "UPSERT_STREAMED_EVENTS"; payload: EventResponse[] }
  | { type: "STREAM_ERROR"; payload: string }
  | { type: "STREAM_STATUS"; payload: { isStreaming: boolean } }
  | { type: "SELECT_EVENTS"; payload: EventResponse[] }
  | { type: "SET_SEARCH_RADIUS"; payload: number | null }

type EventsState = {
  eventsByDate: EventsByDate
  isStreaming: boolean
  selectedEvents: EventResponse[]
  error: string | null
  /** The radius (miles) the current/last search actually used. */
  searchRadius: number | null
}

function sameEvent(a: EventResponse, b: EventResponse): boolean {
  if (a.id === b.id) return true

  const aVenue = a.venue?.name ?? ""
  const bVenue = b.venue?.name ?? ""
  const aPerformer = a.performers?.[0]?.id ?? ""
  const bPerformer = b.performers?.[0]?.id ?? ""

  const exactMatch =
    (a.dates ?? "") === (b.dates ?? "") &&
    aVenue === bVenue &&
    aPerformer === bPerformer

  if (exactMatch) return true

  // Safety net for same-source duplicate listings that differ in title (and
  // therefore in which performer lands first) -- e.g. Ticketmaster's "La
  // Luz" vs. "La Luz w/ Spacemoth" for the same show. The backend already
  // collapses these within a date window (see apps/api/src/events/reconcile.rs
  // merge_same_source_duplicate); this only catches whatever slips past
  // that, e.g. a duplicate split across two windows.
  if (!aVenue || !bVenue || normalizeVenueName(aVenue) !== normalizeVenueName(bVenue)) {
    return false
  }

  const aDay = eventDateKey(a)
  return (
    aDay !== "unknown" && aDay === eventDateKey(b) && performerIdsOverlap(a, b)
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
        searchRadius: null,
      }

    case "STREAM_STATUS":
      return {
        ...state,
        isStreaming: action.payload.isStreaming,
      }

    case "SET_SEARCH_RADIUS":
      return {
        ...state,
        searchRadius: action.payload,
      }

    case "STREAM_ERROR":
      return {
        ...state,
        isStreaming: false,
        error: action.payload,
      }

    case "UPSERT_STREAMED_EVENTS": {
      if (action.payload.length === 0) return state

      const nextByDate = { ...state.eventsByDate }
      const touchedDateKeys = new Set<string>()

      for (const event of action.payload) {
        const dateKey = eventDateKey(event)
        const existingBucket = nextByDate[dateKey] ?? []

        const existingIndex = existingBucket.findIndex((existing) =>
          sameEvent(existing, event)
        )

        // A later emission for the same event (by sameEvent's identity)
        // always carries at least as much information as an earlier one --
        // e.g. the cross-source reconciliation pass re-emitting a
        // Ticketmaster event with rank/predictedAttendance attached -- so
        // replace rather than discard on a match.
        nextByDate[dateKey] =
          existingIndex === -1
            ? [...existingBucket, event]
            : existingBucket.map((existing, i) =>
                i === existingIndex ? event : existing
              )
        touchedDateKeys.add(dateKey)
      }

      // Sort once per touched date bucket rather than once per event -- a
      // batch commonly carries many events for the same day (see
      // eventsProvider.tsx's rAF-batched dispatch).
      for (const dateKey of touchedDateKeys) {
        nextByDate[dateKey] = sortEvents(nextByDate[dateKey])
      }

      return {
        ...state,
        eventsByDate: nextByDate,
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
  searchRadius: null,
}

export {
  type EventsState,
  eventsReducer,
  initialEventsState,
  sameEvent,
  sortEvents,
}
