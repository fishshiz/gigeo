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

type StreamConcertsParams = {
  latitude: number
  longitude: number
  radius: number
  start: string
  end: string
}

type EventsContextValue = EventsState & {
  streamEvents: (params: StreamConcertsParams) => Promise<void>
  cancelStream: () => void
  selectEvents: (events: EventResponse[]) => void
  resetEvents: () => void
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

  const cancelStream = useCallback(() => {
    abortRef.current?.abort()
    abortRef.current = null
    dispatch({ type: "STREAM_STATUS", payload: { isStreaming: false } })
  }, [])

  const resetEvents = useCallback(() => {
    dispatch({ type: "RESET_EVENTS" })
  }, [])

  const selectEvents = useCallback((events: EventResponse[]) => {
    dispatch({ type: "SELECT_EVENTS", payload: events })
  }, [])

  const streamEvents = useCallback(async (params: StreamConcertsParams) => {
    abortRef.current?.abort()

    const controller = new AbortController()
    abortRef.current = controller

    dispatch({ type: "RESET_EVENTS" })
    dispatch({ type: "STREAM_STATUS", payload: { isStreaming: true } })

    try {
      const response = await fetch(buildConcertStreamUrl(params), {
        method: "GET",
        headers: {
          Accept: "application/x-ndjson",
        },
        signal: controller.signal,
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
          }
        }

        buffer += decoder.decode()

        const trailing = buffer.trim()
        if (trailing) {
          const event = JSON.parse(trailing) as EventResponse
          dispatch({ type: "UPSERT_STREAMED_EVENT", payload: event })
        }
      } finally {
        reader.releaseLock()
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
  }, [])

  const value = useMemo(
    () => ({
      ...state,
      streamEvents,
      cancelStream,
      resetEvents,
      selectEvents,
    }),
    [state, streamEvents, cancelStream, resetEvents, selectEvents]
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
