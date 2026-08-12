import { describe, expect, it } from "vitest"
import {
  buildEventFeatureCollection,
  eventIconColorExpression,
  resolveEventsByIds,
} from "./mapbox"
import type { EventResponse, EventsByDate } from "../hooks/eventsStream"

function makeEvent(overrides: Partial<EventResponse> = {}): EventResponse {
  return {
    id: "1",
    name: "Test Event",
    images: [],
    dates: "2026-08-01T20:00:00Z",
    source: "ticketmaster",
    venue: {
      name: "Test Venue",
      location: { latitude: "43.65", longitude: "-70.25" },
      images: [],
    },
    ...overrides,
  }
}

describe("buildEventFeatureCollection", () => {
  it("returns an empty FeatureCollection for no events", () => {
    expect(buildEventFeatureCollection({})).toEqual({
      type: "FeatureCollection",
      features: [],
    })
  })

  it("builds a Point feature from an event with venue location", () => {
    const event = makeEvent({
      id: "e1",
      name: "Big Show",
      venue: {
        name: "The Venue",
        location: { latitude: "43.65", longitude: "-70.25" },
        images: [],
      },
    })
    const result = buildEventFeatureCollection({ "2026-08-01": [event] })

    expect(result.features).toHaveLength(1)
    expect(result.features[0]).toEqual({
      type: "Feature",
      properties: {
        description: "Big Show",
        venue: "The Venue",
        color: "#373630",
        id: "e1",
        selected: "false",
      },
      geometry: {
        type: "Point",
        coordinates: [-70.25, 43.65],
      },
    })
  })

  it("skips events with no venue", () => {
    const event = makeEvent({ venue: undefined })
    expect(buildEventFeatureCollection({ "2026-08-01": [event] })).toEqual({
      type: "FeatureCollection",
      features: [],
    })
  })

  it("skips events with a venue but no location", () => {
    const event = makeEvent({ venue: { name: "No Geo Venue", images: [] } })
    expect(buildEventFeatureCollection({ "2026-08-01": [event] })).toEqual({
      type: "FeatureCollection",
      features: [],
    })
  })

  it("includes events from every date bucket", () => {
    const eventsByDate: EventsByDate = {
      "2026-08-01": [makeEvent({ id: "a" })],
      "2026-08-02": [makeEvent({ id: "b" })],
    }
    const result = buildEventFeatureCollection(eventsByDate)
    const ids = result.features.map((f) => f.properties?.id)
    expect(ids.sort()).toEqual(["a", "b"])
  })
})

describe("eventIconColorExpression", () => {
  it("matches a selected event by id", () => {
    const expression = eventIconColorExpression(
      [makeEvent({ id: "e1" })],
      "red",
      "yellow"
    )
    // ["case", ["in", ["get", "id"], ["literal", [...ids]]], selectedColor, ...]
    expect(expression[1]).toEqual(["in", ["get", "id"], ["literal", ["e1"]]])
    expect(expression[2]).toBe("yellow")
  })

  it("matches by shared venue name for cluster highlighting", () => {
    const expression = eventIconColorExpression(
      [makeEvent({ id: "e1", venue: { name: "Shared Venue", images: [] } })],
      "red",
      "yellow"
    )
    expect(expression[3]).toEqual([
      "in",
      ["get", "clusterVenue"],
      ["literal", ["Shared Venue"]],
    ])
  })

  it("falls back to the default color when nothing is selected", () => {
    const expression = eventIconColorExpression([], "red", "yellow")
    expect(expression[expression.length - 1]).toBe("red")
    expect(expression[1]).toEqual(["in", ["get", "id"], ["literal", []]])
  })
})

describe("resolveEventsByIds", () => {
  it("resolves a single matching event", () => {
    const event = makeEvent({ id: "e1" })
    const eventsByDate: EventsByDate = { "2026-08-01": [event] }
    expect(resolveEventsByIds(eventsByDate, ["e1"])).toEqual([event])
  })

  it("resolves matching events across multiple date buckets", () => {
    const a = makeEvent({ id: "a" })
    const b = makeEvent({ id: "b" })
    const c = makeEvent({ id: "c" })
    const eventsByDate: EventsByDate = {
      "2026-08-01": [a, c],
      "2026-08-02": [b],
    }
    const result = resolveEventsByIds(eventsByDate, ["a", "b"])
    expect(result.map((e) => e.id).sort()).toEqual(["a", "b"])
  })

  it("returns an empty array when no ids match", () => {
    const eventsByDate: EventsByDate = {
      "2026-08-01": [makeEvent({ id: "a" })],
    }
    expect(resolveEventsByIds(eventsByDate, ["nonexistent"])).toEqual([])
  })

  it("returns an empty array for an empty id list", () => {
    const eventsByDate: EventsByDate = {
      "2026-08-01": [makeEvent({ id: "a" })],
    }
    expect(resolveEventsByIds(eventsByDate, [])).toEqual([])
  })
})
