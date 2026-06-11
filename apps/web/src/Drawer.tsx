import { EventCard } from "./EventCard"
import { EventDetails } from "./EventDetails"
import { XIcon } from "lucide-react"
import { DateSlider } from "./DateSlider"
import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"
import { useEventsContext } from "./providers/eventsProvider"
import { useMediaQuery } from "usehooks-ts"
import { useEffect, useRef, type Ref } from "react"
import { useTopMostVisibleInScrollContainer } from "./hooks/listItemObserver"
import { formatDate } from "./lib/formats"
import { VenueDetails } from "./VenueDetails"
import { EventFilter } from "./EventFilter"

const DrawerWrapper = ({
  drawerOpen,
  setDrawerOpen,
}: {
  drawerOpen: boolean
  setDrawerOpen: (open: boolean) => void
}) => {
  const { eventsByDate, selectedEvents } = useEventsContext()
  const eventListRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!eventListRef.current) return
    setDrawerOpen(true)
  }, [eventsByDate])
  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })
  const handleDateChange = (date: string) => {
    if (eventListRef && eventListRef.current) {
      const target = eventListRef.current.querySelector(`#a${date}`)

      if (target) {
        target.scrollIntoView({
          block: "start",
          behavior: "instant",
          // @ts-ignore
          container: "nearest",
        })
      }
    }
  }

  const { topMostId, registerItem } = useTopMostVisibleInScrollContainer(
    eventListRef,
    {
      offsetTop: 0,
    }
  )
  const entries = Object.entries(eventsByDate).sort()

  return (
    <DrawerContent
      isOpen={drawerOpen}
      closeDrawer={() => setDrawerOpen(false)}
      isBlurred={false}
      notch={isDesktop ? false : true}
      side={isDesktop ? "left" : "bottom"}
      className="z-10 flex h-[50dvh] w-full max-w-md flex-col bg-white"
    >
      {!selectedEvents.length && eventsByDate && (
        <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-white">
          <div className="mb-2 flex w-full justify-between">
            <h2 className="text-bold text-xl">Events</h2>
            <DrawerClose variant="quiet" onClick={() => setDrawerOpen(false)}>
              <XIcon />
            </DrawerClose>
          </div>
          <EventFilter />
          <DateSlider
            dates={entries.map(([key]) => key)}
            activeDateId={topMostId}
            onSelect={handleDateChange}
          />
        </DrawerHeader>
      )}
      <DrawerBody className="flex-5 overflow-y-scroll p-0">
        {selectedEvents.length === 1 ? (
          <EventDetails eventData={selectedEvents[0]} />
        ) : selectedEvents.length ? (
          <VenueDetails events={selectedEvents} />
        ) : (
          <div className="overflow-y-scroll scroll-smooth" ref={eventListRef}>
            {entries.map(([date, events]) => (
              <div key={date}>
                <DateAnchor date={date} ref={registerItem(date)} />
                <ul className="flex w-full flex-col justify-between gap-4 pt-4">
                  {events.map((event) => (
                    <li className="relative" key={event.id}>
                      <EventCard
                        key={event.id}
                        event={event}
                        date={
                          <span className="text-gray-600">{event.dates}</span>
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

const DateAnchor = ({ date, ref }: { date: string; ref: Ref<any> }) => {
  const formattedDate = formatDate(date)
  return (
    <>
      <div id={`a${date}`} />
      <div
        ref={ref}
        className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700) dark:text-(--color-text-primary-dark-700)"
      >
        <h3>{formattedDate}</h3>
      </div>
    </>
  )
}

export { DrawerWrapper }
