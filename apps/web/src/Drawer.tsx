import { EventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { EventDetails } from "./EventDetails"
import { PanelLeftCloseIcon } from "lucide-react"
import { DateSlider } from "./DateSlider"
import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"
import { useEventsContext } from "./providers/eventsProvider"
import { formatDate } from "./lib/formats"
import { useMediaQuery } from "usehooks-ts"
import { useEffect, useRef } from "react"
import type { DateValue } from "@internationalized/date"

const DrawerWrapper = ({
  drawerOpen,
  setDrawerOpen,
}: {
  drawerOpen: boolean
  setDrawerOpen: (open: boolean) => void
}) => {
  const { eventsByDate } = useEventsContext()
  const { selectedEvent, dateRange } = useEvents()
  const eventListRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setDrawerOpen(true)
  }, [eventsByDate])
  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })
  const handleDateChange = (date: DateValue) => {
    if (eventListRef && eventListRef.current) {
      const target = eventListRef.current.querySelector(
        `#${date
          .toDate("UTC")
          .toLocaleDateString("en-US", {
            month: "long",
            day: "2-digit",
          })
          .replace(" ", "")
          .toLocaleLowerCase()}`
      )
      console.log(
        target,
        `#${date
          .toDate("UTC")
          .toLocaleDateString("en-US", {
            month: "long",
            day: "2-digit",
          })
          .replace(" ", "")
          .toLocaleLowerCase()}`
      )
      if (target) {
        target.scrollIntoView({ block: "start", behavior: "instant" })
      }
    }
  }

  const options = {
    root: null, // Use the browser viewport
    threshold: 0.5, // Trigger when 50% of the element is visible
  }

  const callback = (entries: any[]) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        // Logic for visible element (e.g., active class, lazy loading)
      }
    })
  }

  const entries = Object.entries(eventsByDate).sort()
  const formatDateId = (date: string) =>
    date.toLocaleLowerCase().replace(" ", "")
  const datesRef = useRef(new Map())
  const observer = new IntersectionObserver(callback, options)
  datesRef.current.forEach((section) => {
    observer.observe(section)
  })

  return (
    <DrawerContent
      isOpen={drawerOpen}
      closeDrawer={() => setDrawerOpen(false)}
      isBlurred={false}
      notch={isDesktop ? false : true}
      side={isDesktop ? "left" : "bottom"}
      className="z-10 flex h-[50dvh] w-full max-w-md flex-col bg-white"
    >
      {!selectedEvent && eventsByDate && (
        <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-white">
          <DrawerClose onClick={() => setDrawerOpen(false)}>
            <PanelLeftCloseIcon />
          </DrawerClose>
          {/* <FilterSection
                handleDateChange={handleDateChange}
                events={eventsByDate}
              /> */}
          <DateSlider dateRange={dateRange} onSelect={handleDateChange} />
        </DrawerHeader>
      )}
      <DrawerBody className="flex-5 overflow-y-scroll p-0">
        {selectedEvent ? (
          <EventDetails eventData={selectedEvent} />
        ) : (
          <div className="overflow-y-scroll scroll-smooth" ref={eventListRef}>
            {entries.map(([date, events]) => (
              <div key={formatDateId(date)}>
                <div
                  id={date.replace(" ", "").toLocaleLowerCase()}
                  ref={(node) => {
                    if (node) {
                      datesRef.current.set(
                        date.replace(" ", "").toLocaleLowerCase(),
                        node
                      )
                    } else {
                      datesRef.current.delete(
                        date.replace(" ", "").toLocaleLowerCase()
                      )
                    }
                  }}
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
                            {formatDate(event.dates || "")}
                          </span>
                        }
                      />
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}
      </DrawerBody>
    </DrawerContent>
  )
}

export { DrawerWrapper }
