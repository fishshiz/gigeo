import { EventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { useState } from "react"
const Drawer = () => {
  const eventsContext = useEvents()
  const { events } = eventsContext

  const [drawerOpen, setDrawerOpen] = useState(false)

  return (
    <div className="absolute top-20 bottom-0 left-0 z-10 w-full flex-auto overflow-y-scroll bg-white p-4 shadow-xl sm:top-0 sm:w-64">
      {events.map((event) => (
        <EventCard key={event.id} event={event} />
      ))}
    </div>
  )
}

export { Drawer }
