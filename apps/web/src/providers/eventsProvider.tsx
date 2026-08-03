/* eslint-disable react-refresh/only-export-components */
import React, {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useReducer,
  useRef,
} from "react"

import {
  type EventsState,
  eventsReducer,
  initialEventsState,
} from "../reducers/events"
import { type EventResponse } from "@/hooks/eventsStream"

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
}

type StreamConcertsParams = StreamConcertsInput & {
  radius: number
}

type EventsContextValue = EventsState & {
  streamEvents: (params: StreamConcertsInput) => Promise<void>
  cancelStream: () => void
  selectEvents: (events: EventResponse[]) => void
  resetEvents: () => void
  /** The radius (miles) the current/last search actually used. */
  searchRadius: number | null
  /** Whether searchRadius went beyond the base tier to find results. */
  radiusExpanded: boolean
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

            const event = JSON.parse(trimmed) as EventResponse
            dispatch({ type: "UPSERT_STREAMED_EVENT", payload: event })
            count++
          }
        }

        buffer += decoder.decode()

        const trailing = buffer.trim()
        if (trailing) {
          const event = JSON.parse(trailing) as EventResponse
          dispatch({ type: "UPSERT_STREAMED_EVENT", payload: event })
          count++
        }
      } finally {
        reader.releaseLock()
      }

      return count
    },
    []
  )

  const streamEvents = useCallback(
    async (params: StreamConcertsInput) => {
      abortRef.current?.abort()

      const controller = new AbortController()
      abortRef.current = controller

      dispatch({ type: "RESET_EVENTS" })
      dispatch({ type: "STREAM_STATUS", payload: { isStreaming: true } })
      setSearchRadius(null)

      try {
        let totalCount = 0

        for (const radius of RADIUS_TIERS) {
          if (controller.signal.aborted) return

          const tierCount = await runStream(
            { ...params, radius },
            controller.signal
          )
          totalCount += tierCount
          setSearchRadius(radius)

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

  const value = useMemo(
    () => ({
      ...state,
      streamEvents,
      cancelStream,
      resetEvents,
      selectEvents,
      searchRadius,
      radiusExpanded,
    }),
    [
      state,
      streamEvents,
      cancelStream,
      resetEvents,
      selectEvents,
      searchRadius,
      radiusExpanded,
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
