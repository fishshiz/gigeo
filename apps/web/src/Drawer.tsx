import { EventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { EventDetails } from "./EventDetails"
import { ChevronUpIcon } from "lucide-react"
import { DateSlider } from "./DateSlider"
import {
  DrawerTrigger,
  DrawerBody,
  DrawerContent,
  DrawerHeader,
} from "@workspace/ui/components/ui/Drawer"

import {
  Disclosure,
  DisclosureHeader,
  DisclosurePanel,
} from "@workspace/ui/components/ui/Disclosure"

import type { Event } from "./lib/types"
import { formatDate } from "./lib/formats"
import { EventFilter } from "./EventFilter"
import { useMediaQuery } from "usehooks-ts"
import { useState, useEffect, useRef } from "react"

const DrawerWrapper = () => {
  const eventsContext = useEvents()
  const [drawerOpen, setDrawerOpen] = useState(true)
  const eventListRef = useRef<HTMLDivElement>(null)
  const { events, selectedEvent } = eventsContext
  useEffect(() => {
    setDrawerOpen(true)
  }, [events])
  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })
  const handleDateChange = (date: string) => {
    if (eventListRef && eventListRef.current) {
      const target = eventListRef.current.querySelector(
        `#${date.replace(" ", "").toLocaleLowerCase()}`
      )
      if (target) {
        target.scrollIntoView({ block: "start", behavior: "instant" })
      }
    }
  }
  return (
    <>
      {drawerOpen ? (
        <DrawerContent
          isOpen={drawerOpen}
          closeDrawer={() => setDrawerOpen(false)}
          isBlurred={false}
          notch={true}
          side={isDesktop ? "left" : "bottom"}
          className="z-10 flex h-[80dvh] w-full max-w-md flex-col bg-white sm:h-full"
        >
          {!selectedEvent && events && (
            <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-white">
              <FilterSection
                handleDateChange={handleDateChange}
                events={events}
              />
            </DrawerHeader>
          )}
          <DrawerBody className="flex-5 overflow-y-scroll p-0">
            {selectedEvent ? (
              <EventDetails eventData={selectedEvent} />
            ) : (
              <EventList events={events} parentRef={eventListRef} />
            )}
          </DrawerBody>
        </DrawerContent>
      ) : (
        <DrawerTrigger
          onClick={() => setDrawerOpen(true)}
          className="shadow:lg absolute bottom-0 z-10 flex w-full items-center justify-center border-t border-gray-300 bg-white p-0 p-4 text-sm font-medium text-gray-700 sm:bottom-[50%] sm:left-0 sm:w-auto"
        >
          <ChevronUpIcon className="stroke-gray-700" />
          <span>Tap to open drawer</span>
        </DrawerTrigger>
      )}
    </>
  )
}

const EventList = ({
  events,
  parentRef,
}: {
  events: Record<string, Event[]>
  parentRef: React.RefObject<HTMLDivElement | null>
}) => {
  const entries = Object.entries(events).sort()
  const formatDateId = (date: string) =>
    date.toLocaleLowerCase().replace(" ", "")
  return (
    <div className="overflow-y-scroll scroll-smooth" ref={parentRef}>
      {entries.map(([date, events]) => (
        <div key={formatDateId(date)}>
          <div
            id={formatDateId(date)}
            className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700) dark:text-(--color-text-primary-dark-700)"
          >
            <h3>{date}</h3>
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

const FilterSection = ({
  handleDateChange,
  events,
}: {
  handleDateChange: (date: string) => void
  events: Record<string, Event[]>
}) => {
  return (
    <Disclosure>
      <DisclosureHeader>Filter</DisclosureHeader>
      <DisclosurePanel>
        <EventFilter />
        <DateSlider
          dates={Object.keys(events).sort()}
          handleChange={handleDateChange}
        />
      </DisclosurePanel>
    </Disclosure>
  )
}

export { DrawerWrapper }
