import { useEffect, useMemo, useRef, useState } from "react"

export type EventsByDate = Record<string, EventResponse[]>

export type EventResponse = {
  id: string
  name: string
  venue?: {
    name?: string
    location?: {
      latitude?: string
      longitude?: string
    }
    city?: string
  } | null
  images: Array<{
    ratio?: string
    url: string
    width?: number
    height?: number
    fallback?: boolean
  }>
  dates?: string | null
  datesPretty?: string | null
  classifications?: { segment: { id: string; name: string } }[] | null
  attractions?: Array<{
    id?: string
    name?: string
  }> | null
  url?: string | null
  priceRanges?: Array<{
    currency: string
    min: number
    max: number
  }> | null
}

type StreamConcertsParams = {
  latitude: number
  longitude: number
  radius: number
  start: string
  end: string
  signal?: AbortSignal
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

export async function* streamConcerts(
  params: StreamConcertsParams
): AsyncGenerator<EventResponse, void, unknown> {
  const response = await fetch(buildConcertStreamUrl(params), {
    method: "GET",
    headers: {
      Accept: "application/x-ndjson",
    },
    signal: params.signal,
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
        yield JSON.parse(trimmed) as EventResponse
      }
    }

    buffer += decoder.decode()

    const trimmed = buffer.trim()
    if (trimmed) {
      yield JSON.parse(trimmed) as EventResponse
    }
  } finally {
    reader.releaseLock()
  }
}
type Props = {
  latitude: number
  longitude: number
  radius: number
  start: string
  end: string
}
export function useConcertStreamBatched({
  latitude,
  longitude,
  radius,
  start,
  end,
}: Props) {
  const [events, setEvents] = useState<EventResponse[]>([])
  const [isStreaming, setIsStreaming] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const pendingRef = useRef<EventResponse[]>([])
  const flushTimerRef = useRef<number | null>(null)

  const key = useMemo(
    () => JSON.stringify({ latitude, longitude, radius, start, end }),
    [latitude, longitude, radius, start, end]
  )

  useEffect(() => {
    const controller = new AbortController()
    let cancelled = false

    const flush = () => {
      if (pendingRef.current.length === 0) return
      const chunk = pendingRef.current
      pendingRef.current = []
      setEvents((prev) => [...prev, ...chunk])
    }

    const scheduleFlush = () => {
      if (flushTimerRef.current != null) return
      flushTimerRef.current = window.setTimeout(() => {
        flushTimerRef.current = null
        flush()
      }, 75)
    }

    setEvents([])
    setError(null)
    setIsStreaming(true)
    ;(async () => {
      try {
        for await (const event of streamConcerts({
          latitude,
          longitude,
          radius,
          start,
          end,
          signal: controller.signal,
        })) {
          if (cancelled) return
          pendingRef.current.push(event)
          scheduleFlush()
        }

        flush()
      } catch (err) {
        if (controller.signal.aborted) return
        setError(err instanceof Error ? err.message : "Unknown stream error")
      } finally {
        flush()
        if (!cancelled) {
          setIsStreaming(false)
        }
      }
    })()

    return () => {
      cancelled = true
      controller.abort()
      if (flushTimerRef.current != null) {
        clearTimeout(flushTimerRef.current)
      }
    }
  }, [key, latitude, longitude, radius, start, end])

  return { events, isStreaming, error }
}
