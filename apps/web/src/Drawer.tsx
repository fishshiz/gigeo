import { EventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { EventDetails } from "./EventDetails"
import { ChevronUpIcon } from "lucide-react"
import {
  Drawer as Cra,
  DrawerTrigger,
  DrawerBody,
  DrawerContent,
  DrawerHeader,
} from "@workspace/ui/components/ui/Drawer"

import type { Event } from "./lib/types"
import { formatDate } from "./lib/formats"
import { ChevronRight } from "lucide-react"
import { Link } from "@workspace/ui/components/ui/Link"
import { EventFilter } from "./EventFilter"
import { useMediaQuery } from "usehooks-ts"
import { useState, useEffect } from "react"

const Drawer = () => {
  const eventsContext = useEvents()
  const [drawerOpen, setDrawerOpen] = useState(true)
  const { events } = eventsContext
  useEffect(() => {
    setDrawerOpen(true)
  }, [events])
  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })
  return (
    <>
      {drawerOpen ? (
        <Cra isOpen={drawerOpen} onOpenChange={setDrawerOpen}>
          <DrawerContent
            isBlurred={false}
            notch={true}
            side={isDesktop ? "left" : "bottom"}
            className="z-10 h-[80dvh] w-full overflow-y-scroll bg-white sm:h-full"
          >
            <DrawerHeader className="sticky top-0 z-10 my-2 bg-white">
              <EventFilter />
            </DrawerHeader>
            <DrawerBody>
              {eventsContext.selectedEvent ? (
                <EventDetails eventData={eventsContext.selectedEvent} />
              ) : (
                <EventList events={events} />
              )}
            </DrawerBody>
          </DrawerContent>
        </Cra>
      ) : (
        <DrawerTrigger
          onClick={() => setDrawerOpen(true)}
          className="absolute bottom-0 z-10 w-full bg-white"
        >
          <ChevronUpIcon />
          <span>Tap to open drawer</span>
        </DrawerTrigger>
      )}
    </>
  )
}

const EventList = ({ events }: { events: Record<string, Event[]> }) => {
  const entries = Object.entries(events).sort(([dateA], [dateB]) =>
    dateA.localeCompare(dateB)
  )
  const formatDateId = (date: string) =>
    date.toLocaleLowerCase().replace(" ", "")
  return (
    <div className="overflow-y-scroll scroll-smooth">
      {entries.map(([date, events], idx) => (
        <div key={formatDateId(date)}>
          <div
            id={formatDateId(date)}
            className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) p-4 dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700) dark:text-(--color-text-primary-dark-700)"
          >
            <h3>{date}</h3>
            {idx < entries.length - 1 && (
              <Link href={`#${formatDateId(entries[idx + 1][0])}`}>
                <ChevronRight />
              </Link>
            )}
          </div>
          <ul className="flex w-full flex-col justify-between gap-4 pt-4">
            {events.map((event) => (
              <li className="relative" key={event.id}>
                <EventCard
                  key={event.id}
                  event={event}
                  date={
                    <span className="text-gray-600">
                      {formatDate(event.dates)}
                    </span>
                  }
                />
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  )
}

export { Drawer }
