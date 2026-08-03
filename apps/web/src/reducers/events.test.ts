import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import {
  eventDateKey,
  eventsReducer,
  initialEventsState,
  sameEvent,
  sortEvents,
  type EventsState,
} from "./events"
import type { EventResponse } from "../hooks/eventsStream"

function makeEvent(overrides: Partial<EventResponse> = {}): EventResponse {
  return {
    id: "1",
    name: "Test Event",
    images: [],
    dates: "2026-08-01T20:00:00Z",
    source: "ticketmaster",
    ...overrides,
  }
}

describe("eventDateKey", () => {
  // Pinned rather than relying on the runner's own default timezone: the
  // whole point of these cases is to exercise UTC-vs-local calendar-day
  // boundaries, which only show up in a timezone behind UTC (a CI runner
  // that happens to default to UTC would silently pass a broken
  // implementation, since there'd be no offset to shift across).
  beforeEach(() => {
    vi.stubEnv("TZ", "America/New_York")
  })

  afterEach(() => {
    vi.unstubAllEnvs()
  })

  it("returns the YYYY-MM-DD portion of a valid date", () => {
    expect(eventDateKey(makeEvent({ dates: "2026-08-01T20:00:00Z" }))).toBe(
      "2026-08-01"
    )
  })

  it("returns 'unknown' when dates is missing", () => {
    expect(eventDateKey(makeEvent({ dates: null }))).toBe("unknown")
  })

  it("returns 'unknown' when dates is not a parseable date", () => {
    expect(eventDateKey(makeEvent({ dates: "not-a-date" }))).toBe("unknown")
  })

  it("buckets a late-evening show under its local calendar day, not the next UTC day", () => {
    // Regression test: an 11pm Eastern show is 3am UTC the *next* day.
    // The previous implementation extracted the UTC calendar day
    // (toISOString().substring(0, 10)), which pushed this into
    // "2026-08-02" even though the show itself is on Aug 1 locally.
    expect(eventDateKey(makeEvent({ dates: "2026-08-02T03:00:00Z" }))).toBe(
      "2026-08-01"
    )
  })

  it("does not shift an early show that already falls on the same UTC day", () => {
    // 4pm Eastern -- well clear of the UTC day boundary either way, so
    // this should be unaffected by the local-vs-UTC change.
    expect(eventDateKey(makeEvent({ dates: "2026-08-01T20:00:00Z" }))).toBe(
      "2026-08-01"
    )
  })

  it("returns a bare YYYY-MM-DD date unchanged, without routing it through Date", () => {
    // Ticketmaster events with only a localDate (no time) come through as
    // a bare date string. Parsing that as `Date` would anchor it to UTC
    // midnight, which reads back as the *previous* day in any timezone
    // behind UTC -- e.g. UTC midnight Aug 1 is 8pm July 31 Eastern.
    expect(eventDateKey(makeEvent({ dates: "2026-08-01" }))).toBe(
      "2026-08-01"
    )
  })

  it("treats a timezone-less local datetime as already-local, not UTC", () => {
    // normalize_event's localDate+localTime fallback produces a string
    // with no "Z"/offset, which JS parses as local wall-clock time
    // already -- no conversion should happen here regardless of the
    // runner's timezone.
    expect(eventDateKey(makeEvent({ dates: "2026-08-01T23:30:00" }))).toBe(
      "2026-08-01"
    )
  })
})

describe("sameEvent", () => {
  it("is true when ids match, regardless of other fields", () => {
    const a = makeEvent({ id: "1", dates: "2026-08-01T20:00:00Z" })
    const b = makeEvent({ id: "1", dates: "2026-09-01T20:00:00Z" })
    expect(sameEvent(a, b)).toBe(true)
  })

  it("is true for different ids with matching date, venue, and performer", () => {
    // Ticketmaster returns the same show under different ids across
    // overlapping date windows; this is the whole point of sameEvent.
    const a = makeEvent({
      id: "tm-1",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "The Venue" },
      performers: [{ id: "artist-1" }],
    })
    const b = makeEvent({
      id: "tm-2",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "The Venue" },
      performers: [{ id: "artist-1" }],
    })
    expect(sameEvent(a, b)).toBe(true)
  })

  it("is false when the venue differs", () => {
    const a = makeEvent({
      id: "tm-1",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "Venue A" },
    })
    const b = makeEvent({
      id: "tm-2",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "Venue B" },
    })
    expect(sameEvent(a, b)).toBe(false)
  })

  it("is false when the date differs", () => {
    const a = makeEvent({ id: "tm-1", dates: "2026-08-01T20:00:00Z" })
    const b = makeEvent({ id: "tm-2", dates: "2026-08-02T20:00:00Z" })
    expect(sameEvent(a, b)).toBe(false)
  })
})

describe("sortEvents", () => {
  it("sorts events by date ascending", () => {
    const later = makeEvent({ id: "1", name: "Later", dates: "2026-08-02T00:00:00Z" })
    const earlier = makeEvent({ id: "2", name: "Earlier", dates: "2026-08-01T00:00:00Z" })
    expect(sortEvents([later, earlier])).toEqual([earlier, later])
  })

  it("sorts events without a date after dated events", () => {
    const dated = makeEvent({ id: "1", name: "Dated", dates: "2026-08-01T00:00:00Z" })
    const undated = makeEvent({ id: "2", name: "Undated", dates: null })
    expect(sortEvents([undated, dated])).toEqual([dated, undated])
  })

  it("breaks ties on the same date by name", () => {
    const b = makeEvent({ id: "1", name: "B Event", dates: "2026-08-01T00:00:00Z" })
    const a = makeEvent({ id: "2", name: "A Event", dates: "2026-08-01T00:00:00Z" })
    expect(sortEvents([b, a])).toEqual([a, b])
  })

  it("does not mutate the input array", () => {
    const events = [
      makeEvent({ id: "1", dates: "2026-08-02T00:00:00Z" }),
      makeEvent({ id: "2", dates: "2026-08-01T00:00:00Z" }),
    ]
    const original = [...events]
    sortEvents(events)
    expect(events).toEqual(original)
  })
})

describe("eventsReducer", () => {
  it("RESET_EVENTS clears state back to initial", () => {
    const dirty: EventsState = {
      eventsByDate: { "2026-08-01": [makeEvent()] },
      isStreaming: true,
      selectedEvents: [makeEvent()],
      error: "boom",
    }
    expect(eventsReducer(dirty, { type: "RESET_EVENTS" })).toEqual(
      initialEventsState
    )
  })

  it("STREAM_STATUS updates isStreaming only", () => {
    const next = eventsReducer(initialEventsState, {
      type: "STREAM_STATUS",
      payload: { isStreaming: true },
    })
    expect(next.isStreaming).toBe(true)
    expect(next.eventsByDate).toBe(initialEventsState.eventsByDate)
  })

  it("STREAM_ERROR sets the error and stops streaming", () => {
    const streaming: EventsState = { ...initialEventsState, isStreaming: true }
    const next = eventsReducer(streaming, {
      type: "STREAM_ERROR",
      payload: "network error",
    })
    expect(next.error).toBe("network error")
    expect(next.isStreaming).toBe(false)
  })

  it("UPSERT_STREAMED_EVENT buckets a new event by its date key", () => {
    const event = makeEvent({ dates: "2026-08-01T20:00:00Z" })
    const next = eventsReducer(initialEventsState, {
      type: "UPSERT_STREAMED_EVENT",
      payload: event,
    })
    expect(next.eventsByDate["2026-08-01"]).toEqual([event])
  })

  it("UPSERT_STREAMED_EVENT collapses a duplicate of the same event into one entry, not two", () => {
    const event = makeEvent({
      id: "tm-1",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "The Venue" },
    })
    const duplicate = makeEvent({
      id: "tm-2",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "The Venue" },
    })

    const afterFirst = eventsReducer(initialEventsState, {
      type: "UPSERT_STREAMED_EVENT",
      payload: event,
    })
    const afterSecond = eventsReducer(afterFirst, {
      type: "UPSERT_STREAMED_EVENT",
      payload: duplicate,
    })

    expect(afterSecond.eventsByDate["2026-08-01"]).toHaveLength(1)
  })

  it("UPSERT_STREAMED_EVENT replaces a matching event with the later payload rather than discarding it", () => {
    // This is what makes cross-source enrichment work: the backend re-emits
    // the same Ticketmaster event id with rank/predictedAttendance attached
    // once PredictHQ reconciliation completes, and that update must actually
    // reach the UI instead of being silently dropped as "already exists".
    const original = makeEvent({
      id: "tm-1",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "The Venue" },
    })
    const enriched = makeEvent({
      id: "tm-1",
      dates: "2026-08-01T20:00:00Z",
      venue: { name: "The Venue" },
      rank: 62,
      predictedAttendance: 900,
    })

    const afterFirst = eventsReducer(initialEventsState, {
      type: "UPSERT_STREAMED_EVENT",
      payload: original,
    })
    const afterSecond = eventsReducer(afterFirst, {
      type: "UPSERT_STREAMED_EVENT",
      payload: enriched,
    })

    expect(afterSecond.eventsByDate["2026-08-01"]).toHaveLength(1)
    expect(afterSecond.eventsByDate["2026-08-01"][0]).toEqual(enriched)
  })

  it("SELECT_EVENTS replaces selectedEvents", () => {
    const events = [makeEvent({ id: "1" }), makeEvent({ id: "2" })]
    const next = eventsReducer(initialEventsState, {
      type: "SELECT_EVENTS",
      payload: events,
    })
    expect(next.selectedEvents).toBe(events)
  })

  it("returns the same state reference for an unknown action", () => {
    // @ts-expect-error intentionally testing the default branch
    expect(eventsReducer(initialEventsState, { type: "NOOP" })).toBe(
      initialEventsState
    )
  })
})
