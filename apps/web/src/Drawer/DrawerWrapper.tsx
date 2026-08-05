import { EventDetails } from "../EventDetails"
import { XIcon } from "lucide-react"
import { motion, useReducedMotion } from "motion/react"
import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"
import { useEventsContext } from "../providers/eventsProvider"
import { useMediaQuery } from "usehooks-ts"
import { useEffect, useCallback, useRef, useState, lazy, Suspense } from "react"
import { VenueDetails } from "../VenueDetails"
import { EventsDrawer, EventsDrawerHeader } from "./EventsDrawer"
import { useTopMostVisibleInScrollContainer } from "../hooks/listItemObserver"
import type { Key } from "react-aria-components/Tabs"
import { useDrawerProvider } from "@/providers/drawerProvider"
import { groupEvents } from "../lib/groupEvents"

const PlaylistsDrawerBody = lazy(() =>
  import("./PlaylistsDrawer").then((m) => ({ default: m.PlaylistsDrawerBody }))
)

const PlaylistDrawerHeader = lazy(() =>
  import("./PlaylistsDrawer").then((m) => ({
    default: m.PlaylistDrawerHeader,
  }))
)

import {
  Tabs,
  TabList,
  Tab,
  TabPanels,
  TabPanel,
} from "@workspace/ui/components/ui/Tabs"
import SpotifyLogo from "../assets/Primary_Logo_Green_CMYK.svg"
import AppleMusicLogo from "../assets/Apple_Music_Icon_RGB_lg_073120.svg"
import { ReactSVG } from "react-svg"
import { Compass } from "lucide-react"

const destinationTabs = [
  { id: "explore", label: "Explore" },
  { id: "spotify", label: "Spotify" },
  { id: "apple", label: "Apple Music" },
] as const

const DestinationIcon = ({
  id,
  size,
}: {
  id: (typeof destinationTabs)[number]["id"]
  size: number
}) => {
  if (id === "explore") return <Compass size={size} />
  if (id === "spotify")
    return (
      <ReactSVG
        className="shrink-0"
        style={{ width: size, height: size }}
        src={SpotifyLogo}
      />
    )
  return (
    <ReactSVG
      className="shrink-0"
      style={{ width: size, height: size }}
      beforeInjection={(svg) => {
        svg.setAttribute("width", String(size))
        svg.setAttribute("height", String(size))
        svg.setAttribute("viewBox", svg.getAttribute("viewBox") || "0 0 24 24")
      }}
      src={AppleMusicLogo}
    />
  )
}

const DrawerWrapper = () => {
  const { eventsByDate, selectedEvents } = useEventsContext()
  const { isDrawerOpen, setIsDrawerOpen } = useDrawerProvider()
  const shouldReduceMotion = useReducedMotion()

  const [activeTab, setActiveTab] = useState<Key>("explore")
  // FIXME: eventListRef is never attached to a DOM node, so the effect below
  // that guards on `eventListRef.current` never runs — "switch to Explore
  // tab on new events" is currently dead code, not working behavior. Left
  // as-is for now; revisit as its own change rather than reviving it as a
  // side effect of the date-scroll fix.
  const eventListRef = useRef<HTMLDivElement>(null)
  const eventsScrollRef = useRef<HTMLDivElement>(null)
  const { topMostId, registerItem } = useTopMostVisibleInScrollContainer(
    eventsScrollRef,
    { offsetTop: 0 }
  )

  const handleDestinationTab = useCallback(
    (tab: Key) => {
      setIsDrawerOpen(true)
      setActiveTab(tab)
    },
    [setIsDrawerOpen]
  )

  const scrollToDate = useCallback((date: string) => {
    const target = eventsScrollRef.current?.querySelector(`#a${date}`)
    target?.scrollIntoView({ block: "start", behavior: "smooth" })
  }, [])

  useEffect(() => {
    // Syncs drawer visibility/active tab to event selection, which is owned
    // by a different provider (eventsProvider) — not derivable at render
    // time without lifting that state up or touching every selectEvents()
    // call site (map marker clicks, drawer list clicks, etc.).
    if (selectedEvents.length) {
      setIsDrawerOpen(true)
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setActiveTab("explore")
    }
  }, [selectedEvents, setIsDrawerOpen])

  useEffect(() => {
    if (!eventListRef.current) return
    handleDestinationTab("explore")
  }, [eventsByDate, setIsDrawerOpen, handleDestinationTab])

  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })

  const entries = Object.entries(eventsByDate).sort()
  // A single group means selectedEvents is either one event, or several
  // showtimes of the *same* event (selected via a GroupedEventCard) — both
  // go to EventDetails. More than one group means genuinely distinct events
  // (e.g. a venue-marker click), which go to VenueDetails.
  const selectedGroups = groupEvents(selectedEvents)

  // Kept in lockstep with the DrawerContent slide transition in the shared
  // Drawer component, so the reserved layout width and the visible slide
  // finish at the same time instead of the map snapping to size afterward.
  const DRAWER_TRANSITION = { duration: 0.15, ease: "easeInOut" as const }
  // Must match the literal "w-[26rem]" on DrawerContent below — Tailwind
  // can't see this value if it's interpolated into a class name, so the
  // two have to be kept in sync by hand.
  const desktopDrawerWidth = "26rem"

  const drawerContent = (
    <DrawerContent
      isOpen={isDrawerOpen}
      closeDrawer={() => setIsDrawerOpen(false)}
      notch={isDesktop ? false : true}
      side={isDesktop ? "left" : "bottom"}
      className={
        isDesktop
          ? "z-10 flex h-full w-[26rem] flex-col overflow-hidden"
          : "z-10 flex h-full w-full flex-col overflow-hidden"
      }
    >
      <DrawerClose
        aria-label="Close drawer"
        className="absolute top-3 right-3 z-20 touch-manipulation max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
        variant="quiet"
        onClick={() => setIsDrawerOpen(false)}
      >
        <XIcon />
      </DrawerClose>

      {!isDesktop && (
        <TabList
          aria-label="Content sections"
          className="flex shrink-0 items-center justify-center gap-1 border-b border-black/10 px-2 pt-1 pr-12 pb-2 dark:border-white/10"
        >
          {destinationTabs.map(({ id, label }) => (
            <Tab
              key={id}
              id={id}
              aria-label={label}
              className="touch-manipulation gap-1.5 px-2 py-3.5"
            >
              <DestinationIcon id={id} size={16} />
              <span className="hidden text-xs font-medium min-[375px]:inline">
                {label}
              </span>
            </Tab>
          ))}
        </TabList>
      )}

      <TabPanels>
        <TabPanel id="explore" className="p-0">
          {!selectedEvents.length && entries.length > 0 && (
            <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-background">
              <EventsDrawerHeader topMostId={topMostId} onSelect={scrollToDate} />
            </DrawerHeader>
          )}
          <DrawerBody
            ref={eventsScrollRef}
            className="h-full flex-1 overflow-y-auto scroll-smooth"
          >
            {selectedGroups.length === 1 ? (
              <EventDetails
                eventData={selectedGroups[0].events[0]}
                otherShowtimes={selectedGroups[0].events.slice(1)}
              />
            ) : selectedGroups.length ? (
              <VenueDetails events={selectedEvents} />
            ) : (
              <EventsDrawer registerItem={registerItem} />
            )}
          </DrawerBody>
        </TabPanel>

        <TabPanel id="spotify" className="flex flex-col">
          <Suspense fallback={null}>
            <PlaylistDrawerHeader />
            <PlaylistsDrawerBody />
          </Suspense>
        </TabPanel>
        <TabPanel
          id="search"
          className="flex items-center justify-center"
        ></TabPanel>
        <TabPanel
          id="settings"
          className="flex items-center justify-center"
        ></TabPanel>
      </TabPanels>
    </DrawerContent>
  )

  return (
    <Tabs
      orientation={isDesktop ? "vertical" : "horizontal"}
      className="h-[40vh] shrink-0 md:h-full"
      selectedKey={activeTab}
      onSelectionChange={(t) => handleDestinationTab(t)}
    >
      {isDesktop && (
        <TabList
          aria-label="Content sections"
          className="h-full flex-col items-center gap-1 border-r border-black/10 py-2 dark:border-white/10"
        >
          {destinationTabs.map(({ id, label }) => (
            <Tab
              key={id}
              id={id}
              aria-label={label}
              className="w-16 touch-manipulation flex-col gap-1 rounded-lg px-2 py-2"
            >
              <DestinationIcon id={id} size={20} />
              <span className="text-[10px] leading-none font-medium">
                {label}
              </span>
            </Tab>
          ))}
        </TabList>
      )}

      {isDesktop ? (
        <motion.div
          className="h-full shrink-0 overflow-hidden"
          initial={false}
          animate={{ width: isDrawerOpen ? desktopDrawerWidth : "0rem" }}
          transition={shouldReduceMotion ? { duration: 0 } : DRAWER_TRANSITION}
        >
          {drawerContent}
        </motion.div>
      ) : (
        drawerContent
      )}
    </Tabs>
  )
}

export { DrawerWrapper }
