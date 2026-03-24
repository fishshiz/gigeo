import { EventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { useState } from "react"
const Drawer = () => {
  const eventsContext = useEvents()
  const { events } = eventsContext

  const [drawerOpen, setDrawerOpen] = useState(false)

  return (
    <div className="w-64 flex-auto shadow-xl">
      {events.map((event) => (
        <EventCard event={event} />
      ))}
    </div>
  )
}

export { Drawer }
