/* eslint-disable react-refresh/only-export-components */
import * as React from "react"
import { type Event } from "@/lib/types"
import {
  type CalendarDate,
  today,
  getLocalTimeZone,
} from "@internationalized/date"
import { type RangeValue } from "react-aria"
import type { EventResponse } from "@/hooks/eventsStream"

const INITIAL_CENTER: [number, number] = [-90.078202, 29.975962]

type EventsProviderState = {
  events: Record<string, Event[]>
  setEvents: (events: Record<string, Event[]>) => void
  selectedEvent: EventResponse | undefined
  setSelectedEvent: (event: EventResponse | undefined) => void
  selectedCoordinates: [number, number]
  setSelectedCoordinates: (coordinates: [number, number]) => void
  dateRange: RangeValue<CalendarDate>
  setDateRange: (date: RangeValue<CalendarDate>) => void
}

type EventsProviderProps = {
  children: React.ReactNode
}

export const EventsProviderContext = React.createContext<
  EventsProviderState | undefined
>(undefined)

export function EventsProvider({ children }: EventsProviderProps) {
  const [events, setEvents] = React.useState<Record<string, Event[]>>({})
  const [selectedEvent, setSelectedEvent] = React.useState<
    EventResponse | undefined
  >(undefined)
  const [selectedCoordinates, setSelectedCoordinates] =
    React.useState<[number, number]>(INITIAL_CENTER)

  const [dateRange, setDateRange] = React.useState<RangeValue<CalendarDate>>({
    start: today(getLocalTimeZone()),
    end: today(getLocalTimeZone()).add({ weeks: 1 }),
  })

  return (
    <EventsProviderContext
      value={{
        events,
        selectedEvent,
        setEvents,
        setSelectedEvent,
        selectedCoordinates,
        setSelectedCoordinates,
        dateRange,
        setDateRange,
      }}
    >
      {children}
    </EventsProviderContext>
  )
}

export const useEvents = () => {
  const context = React.useContext(EventsProviderContext)

  if (context === undefined) {
    throw new Error("useEvents must be used within a EventsProvider")
  }

  return context
}
