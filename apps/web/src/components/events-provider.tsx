/* eslint-disable react-refresh/only-export-components */
import * as React from "react"
import { type Event } from "@/lib/types"

type EventsProviderState = {
  events: Event[]
  setEvents: (events: Event[]) => void
  selectedEvent: Event | undefined
  setSelectedEvent: (event: Event | undefined) => void
}

type EventsProviderProps = {
  children: React.ReactNode
}

export const EventsProviderContext = React.createContext<
  EventsProviderState | undefined
>(undefined)

export function EventsProvider({ children }: EventsProviderProps) {
  const [events, setEvents] = React.useState<Event[]>([])
  const [selectedEvent, setSelectedEvent] = React.useState<Event | undefined>(
    undefined
  )

  return (
    <EventsProviderContext
      value={{ events, selectedEvent, setEvents, setSelectedEvent }}
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
