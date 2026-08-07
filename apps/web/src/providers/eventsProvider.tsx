/* eslint-disable react-refresh/only-export-components */
import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from "react"

import {
  type EventsState,
  eventsReducer,
  initialEventsState,
} from "../reducers/events"
import {
  eventResponseSchema,
  type EventResponse,
} from "@/hooks/eventsStreamSchema"
import { type EventsByDate } from "@/hooks/eventsStream"
import { useSearchProvider } from "./searchProvider"
import { dateRangeToApiParams } from "../lib/dates"

/** Parses one ndjson line against `eventResponseSchema` rather than a bare
 * `JSON.parse(...) as EventResponse` -- a shape mismatch (backend/frontend
 * drift, or genuinely malformed JSON) is logged loudly and the one event
 * is skipped, instead of either silently mis-typing bad data or crashing
 * the whole stream over a single record. */
function parseStreamedEvent(raw: string) {
  let json: unknown
  try {
    json = JSON.parse(raw)
  } catch (err) {
    console.error("Failed to parse a streamed event as JSON", err, raw)
    return null
  }

  const result = eventResponseSchema.safeParse(json)
  if (!result.success) {
    console.error(
      "Streamed event didn't match the expected schema -- backend/frontend drift?",
      result.error,
      json
    )
    return null
  }

  return result.data
}

/** The genres of the performer that triggered `event.matchedArtist`, read
 * off that performer's already-attached canonical-artist enrichment (see
 * `apps/api/src/artists/lookup.rs`) rather than any personalization-
 * specific data -- there isn't any, the "for you" match pipeline only ever
 * tracks a name. Empty when unmatched, or when that performer's
 * enrichment hasn't resolved yet (best-effort, same as everywhere else
 * `enrichment` is read). */
export function matchedPerformerGenres(event: EventResponse): string[] {
  if (!event.matchedArtist) return []
  const performer = event.performers?.find(
    (p) => p.name === event.matchedArtist
  )
  return performer?.enrichment?.genres ?? []
}

export function matchesClassificationFilter(
  event: EventResponse,
  active: Set<string>
): boolean {
  if (active.size === 0) return true
  return (event.classifications ?? []).some(
    (c) => c.primary && c.segment?.name && active.has(c.segment.name)
  )
}

export function matchesForYouFilter(
  event: EventResponse,
  activeArtists: Set<string>,
  activeGenres: Set<string>
): boolean {
  if (activeArtists.size === 0 && activeGenres.size === 0) return true
  if (!event.matchedArtist) return false
  if (activeArtists.has(event.matchedArtist)) return true
  return matchedPerformerGenres(event).some((g) => activeGenres.has(g))
}

// Search radii (miles) tried in order until one returns events. Lets
// sparsely-served (e.g. rural) locations still find something instead of
// permanently showing an empty result at the default radius.
const RADIUS_TIERS = [10, 50, 150] as const
const BASE_RADIUS = RADIUS_TIERS[0]

type StreamConcertsInput = {
  latitude: number
  longitude: number
  start: string
  end: string
  /** Radius tier (miles) to start widening from, skipping any smaller
   * tiers that a previous nearby search already knows come up empty.
   * Defaults to the smallest tier -- a genuinely new location has no such
   * guarantee. */
  startRadius?: number
}

type StreamConcertsParams = StreamConcertsInput & {
  radius: number
}

type EventsContextValue = EventsState & {
  streamEvents: (params: StreamConcertsInput) => Promise<void>
  cancelStream: () => void
  selectEvents: (events: EventResponse[]) => void
  resetEvents: () => void
  /** Re-searches at `coordinates` starting from the smallest search radius tier -- for "search this
   * area", where the user hasn't indicated local event density has
   * changed, just their location. Unlike useNavigateToLocation, this
   * doesn't reset the selected-location label or date range. */
  searchThisArea: (coordinates: [number, number]) => void
  /** The radius (miles) the current/last search actually used. */
  searchRadius: number | null
  /** Whether searchRadius went beyond the base tier to find results. */
  radiusExpanded: boolean
  /** `eventsByDate` narrowed by the active filters (classification + For
   * You) -- what list/map rendering should consume. `eventsByDate` itself
   * stays the raw, unfiltered set so filter-option counts don't shrink as
   * more filters get applied. */
  visibleEventsByDate: EventsByDate
  activeClassifications: Set<string>
  setActiveClassifications: (classifications: Set<string>) => void
  activeForYouArtists: Set<string>
  setActiveForYouArtists: (artists: Set<string>) => void
  activeForYouGenres: Set<string>
  setActiveForYouGenres: (genres: Set<string>) => void
}

function buildConcertStreamUrl(params: StreamConcertsParams) {
  const qs = new URLSearchParams({
    latitude: String(params.latitude),
    longitude: String(params.longitude),
    radius: String(params.radius),
    start: params.start,
    end: params.end,
  })

  return `/api/concerts/stream?${qs.toString()}`
}

const EventsContext = createContext<EventsContextValue | null>(null)
export function EventsProvider({ children }: { children: React.ReactNode }) {
  const [state, dispatch] = useReducer(eventsReducer, initialEventsState)
  const abortRef = useRef<AbortController | null>(null)
  const [activeClassifications, setActiveClassifications] = useState<
    Set<string>
  >(new Set())
  const [activeForYouArtists, setActiveForYouArtists] = useState<Set<string>>(
    new Set()
  )
  const [activeForYouGenres, setActiveForYouGenres] = useState<Set<string>>(
    new Set()
  )
  const visibleEventsByDate = useMemo(() => {
    if (
      activeClassifications.size === 0 &&
      activeForYouArtists.size === 0 &&
      activeForYouGenres.size === 0
    ) {
      return state.eventsByDate
    }

    const result: EventsByDate = {}
    for (const [date, events] of Object.entries(state.eventsByDate)) {
      const filtered = events.filter(
        (event) =>
          matchesClassificationFilter(event, activeClassifications) &&
          matchesForYouFilter(event, activeForYouArtists, activeForYouGenres)
      )
      if (filtered.length) result[date] = filtered
    }
    return result
  }, [
    state.eventsByDate,
    activeClassifications,
    activeForYouArtists,
    activeForYouGenres,
  ])
  const [searchRadius, setSearchRadius] = React.useState<number | null>(null)

  const cancelStream = useCallback(() => {
    abortRef.current?.abort()
    abortRef.current = null
    dispatch({ type: "STREAM_STATUS", payload: { isStreaming: false } })
  }, [])

  const resetEvents = useCallback(() => {
    dispatch({ type: "RESET_EVENTS" })
    setSearchRadius(null)
  }, [])

  const selectEvents = useCallback((events: EventResponse[]) => {
    dispatch({ type: "SELECT_EVENTS", payload: events })
  }, [])

  /** Runs one radius tier's request to completion. Returns the number of
   * events the server sent for this request (pre-dedup). */
  const runStream = useCallback(
    async (params: StreamConcertsParams, signal: AbortSignal) => {
      const response = await fetch(buildConcertStreamUrl(params), {
        method: "GET",
        headers: {
          Accept: "application/x-ndjson",
        },
        signal,
      })

      if (!response.ok) {
        const text = await response.text().catch(() => "")
        throw new Error(`Stream request failed: ${response.status} ${text}`)
      }

      if (!response.body) {
        throw new Error("Response body is not readable")
      }

      const reader = response.body.getReader()
      const decoder = new TextDecoder()
      let buffer = ""
      let count = 0

      try {
        while (true) {
          const { done, value } = await reader.read()

          if (done) break

          buffer += decoder.decode(value, { stream: true })
          const lines = buffer.split("\n")
          buffer = lines.pop() ?? ""

          for (const line of lines) {
            const trimmed = line.trim()
            if (!trimmed) continue

            const event = parseStreamedEvent(trimmed)
            if (!event) continue
            dispatch({ type: "UPSERT_STREAMED_EVENT", payload: event })
            count++
          }
        }

        buffer += decoder.decode()

        const trailing = buffer.trim()
        if (trailing) {
          const event = parseStreamedEvent(trailing)
          if (event) {
            dispatch({ type: "UPSERT_STREAMED_EVENT", payload: event })
            count++
          }
        }
      } finally {
        reader.releaseLock()
      }

      return count
    },
    []
  )

  //FIXME commenting these setState lines out as they violate lint rules https://react.dev/learn/you-might-not-need-an-effect. Need to fix
  const streamEvents = useCallback(
    async (params: StreamConcertsInput) => {
      abortRef.current?.abort()

      const controller = new AbortController()
      abortRef.current = controller

      dispatch({ type: "RESET_EVENTS" })
      dispatch({ type: "STREAM_STATUS", payload: { isStreaming: true } })
      // setSearchRadius(null)

      const startRadius = params.startRadius ?? BASE_RADIUS
      const tiers = RADIUS_TIERS.filter((radius) => radius >= startRadius)

      try {
        let totalCount = 0

        for (const radius of tiers) {
          if (controller.signal.aborted) return

          const tierCount = await runStream(
            { ...params, radius },
            controller.signal
          )
          totalCount += tierCount
          // setSearchRadius(radius)

          if (totalCount > 0) break
        }

        dispatch({ type: "STREAM_STATUS", payload: { isStreaming: false } })
      } catch (err) {
        if (controller.signal.aborted) {
          return
        }

        dispatch({
          type: "STREAM_ERROR",
          payload: err instanceof Error ? err.message : "Unknown stream error",
        })
      } finally {
        if (abortRef.current === controller) {
          abortRef.current = null
        }
      }
    },
    [runStream]
  )

  const radiusExpanded = searchRadius !== null && searchRadius > BASE_RADIUS

  // Reactively (re-)runs the search whenever the selected location or date
  // range changes. Lives here rather than in MapWrapper: neither
  // searchProvider's state nor streamEvents/cancelStream are map-specific --
  // this is just the two providers' own concerns meeting, and previously
  // only met inside MapWrapper because that's where the effect was first
  // written, not because the map needed to own it.
  const { selectedCoordinates, setSelectedCoordinates, dateRange } =
    useSearchProvider()
  const [rendered, setRendered] = useState(false)
  useEffect(() => {
    // Marks first-mount completion so the search effect below skips its
    // initial run; intentional, not a cascading-render risk here.
    // eslint-disable-next-line react-hooks/set-state-in-effect
    setRendered(true)
  }, [])

  const searchThisArea = useCallback(
    (coordinates: [number, number]) => {
      setSelectedCoordinates(coordinates)
    },
    [searchRadius, setSelectedCoordinates]
  )

  const { latitude, longitude, start, end } = {
    latitude: selectedCoordinates[1],
    longitude: selectedCoordinates[0],
    ...dateRangeToApiParams(dateRange.start, dateRange.end),
  }
  useEffect(() => {
    if (!rendered) return
    const startRadius = BASE_RADIUS

    void streamEvents({ latitude, longitude, start, end, startRadius })

    return () => {
      cancelStream()
    }
  }, [latitude, longitude, start, end, streamEvents, cancelStream, rendered])

  const value = useMemo(
    () => ({
      ...state,
      streamEvents,
      cancelStream,
      resetEvents,
      selectEvents,
      searchThisArea,
      searchRadius,
      radiusExpanded,
      visibleEventsByDate,
      activeClassifications,
      setActiveClassifications,
      activeForYouArtists,
      setActiveForYouArtists,
      activeForYouGenres,
      setActiveForYouGenres,
    }),
    [
      state,
      streamEvents,
      cancelStream,
      resetEvents,
      selectEvents,
      searchThisArea,
      searchRadius,
      radiusExpanded,
      visibleEventsByDate,
      activeClassifications,
      activeForYouArtists,
      activeForYouGenres,
    ]
  )

  return (
    <EventsContext.Provider value={value}>{children}</EventsContext.Provider>
  )
}

export function useEventsContext() {
  const ctx = useContext(EventsContext)
  if (!ctx) {
    throw new Error("useEventsContext must be used within EventsProvider")
  }
  return ctx
}
