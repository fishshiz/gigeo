import { EventDetails } from "../EventDetails"
import { XIcon, ChevronDown, ChevronUp } from "lucide-react"
import { motion, useReducedMotion } from "motion/react"
import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"
import { useEventsContext } from "../providers/eventsProvider"
import { useIsMobile } from "../providers/Breakpoint"
import { useEffect, useCallback, useRef, useState, lazy, Suspense } from "react"
import { VenueDetails } from "../VenueDetails"
import { EventsDrawer, EventsDrawerHeader } from "./EventsDrawer"
import { useTopMostVisibleInScrollContainer } from "../hooks/listItemObserver"
import type { Key } from "react-aria-components/Tabs"
import {
  useDrawerProvider,
  type DrawerSnapPoint,
} from "@/providers/drawerProvider"
import { groupEvents } from "../lib/groupEvents"

// Matches AppHeader's h-16 (apps/web/src/Header.tsx) -- kept as a constant
// here rather than measured, since the header's own height never changes.
const HEADER_HEIGHT_PX = 64
const SNAP_FRACTIONS: Record<DrawerSnapPoint, number> = {
  peek: 0.15,
  half: 0.5,
  full: 0.88,
}
const SNAP_ORDER: DrawerSnapPoint[] = ["peek", "half", "full"]

const PlaylistsDrawerBody = lazy(() =>
  import("./PlaylistsDrawer").then((m) => ({ default: m.PlaylistsDrawerBody }))
)

const PlaylistDrawerHeader = lazy(() =>
  import("./PlaylistsDrawer").then((m) => ({
    default: m.PlaylistDrawerHeader,
  }))
)

const AppleMusicDrawerBody = lazy(() =>
  import("./AppleMusicDrawer").then((m) => ({
    default: m.AppleMusicDrawerBody,
  }))
)

const AppleMusicDrawerHeader = lazy(() =>
  import("./AppleMusicDrawer").then((m) => ({
    default: m.AppleMusicDrawerHeader,
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
  const { eventsByDate, selectedEvents, isStreaming } = useEventsContext()
  const { isDrawerOpen, setIsDrawerOpen, snapPoint, setSnapPoint } =
    useDrawerProvider()
  const shouldReduceMotion = useReducedMotion()
  const isDesktop = !useIsMobile()

  const [activeTab, setActiveTab] = useState<Key>("explore")
  const eventsScrollRef = useRef<HTMLDivElement>(null)
  const { topMostId, registerItem } = useTopMostVisibleInScrollContainer(
    eventsScrollRef,
    { offsetTop: 0 }
  )

  const entries = Object.entries(eventsByDate).sort()
  // A single group means selectedEvents is either one event, or several
  // showtimes of the *same* event (selected via a GroupedEventCard) — both
  // go to EventDetails. More than one group means genuinely distinct events
  // (e.g. a venue-marker click), which go to VenueDetails.
  const selectedGroups = groupEvents(selectedEvents)

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
    // call site (map marker clicks, drawer list clicks, etc.). Mobile also
    // jumps to "full" so the just-selected event/venue is actually readable
    // rather than left at whatever height the sheet happened to be at.
    if (selectedEvents.length) {
      setIsDrawerOpen(true)
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setActiveTab("explore")
      if (!isDesktop) setSnapPoint("full")
    }
  }, [selectedEvents, setIsDrawerOpen, isDesktop, setSnapPoint])

  // Switches to the Explore tab when a new search starts (e.g. the caller
  // is on the Spotify/Apple Music tab, then moves the map or changes the
  // date range) so the new results are actually visible rather than
  // silently arriving behind unrelated tab content.
  //
  // Keyed off isStreaming's false->true transition, not eventsByDate:
  // the backend streams results in one at a time (see
  // eventsProvider.tsx's UPSERT_STREAMED_EVENT), so eventsByDate changes
  // dozens of times over the course of a single search -- switching tabs
  // on every one of those would be exactly the kind of repeated,
  // twitchy interruption this behavior is meant to avoid, not cause.
  //
  // Mobile also drives the sheet's height off the same transition: peek
  // the moment a search starts (keep the map visible while results are
  // still uncertain), then rise to half once streaming finishes with
  // results and nothing's selected (selection, handled above, wins if
  // both land in the same commit).
  const wasStreamingRef = useRef(isStreaming)
  useEffect(() => {
    const wasStreaming = wasStreamingRef.current
    wasStreamingRef.current = isStreaming
    if (isStreaming && !wasStreaming) {
      handleDestinationTab("explore")
      if (!isDesktop) setSnapPoint("peek")
    } else if (!isStreaming && wasStreaming) {
      if (!isDesktop && entries.length > 0 && selectedGroups.length === 0) {
        setSnapPoint("half")
      }
    }
  }, [
    isStreaming,
    handleDestinationTab,
    isDesktop,
    setSnapPoint,
    entries.length,
    selectedGroups.length,
  ])

  // Mirrors Breakpoint.tsx's own resize-listener pattern rather than a
  // ResizeObserver -- this only needs the window's own size, not any
  // particular element's.
  const [viewportHeight, setViewportHeight] = useState(() => window.innerHeight)
  useEffect(() => {
    const onResize = () => setViewportHeight(window.innerHeight)
    window.addEventListener("resize", onResize)
    return () => window.removeEventListener("resize", onResize)
  }, [])

  // Pixel snap points for the mobile bottom sheet, ascending
  // [peek, half, full]. Desktop never reads these.
  const sheetContainerHeightPx = viewportHeight - HEADER_HEIGHT_PX
  const snapPointsPx = SNAP_ORDER.map((point) =>
    Math.round(sheetContainerHeightPx * SNAP_FRACTIONS[point])
  )
  const activeSnapIndex = SNAP_ORDER.indexOf(snapPoint)

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
      closeDrawer={
        isDesktop ? () => setIsDrawerOpen(false) : () => setSnapPoint("peek")
      }
      notch={!isDesktop}
      side={isDesktop ? "left" : "bottom"}
      snapPointsPx={isDesktop ? undefined : snapPointsPx}
      activeSnapIndex={isDesktop ? undefined : activeSnapIndex}
      onSnapChange={
        isDesktop ? undefined : (i: number) => setSnapPoint(SNAP_ORDER[i])
      }
      className={
        isDesktop
          ? "z-10 flex h-full w-[26rem] flex-col overflow-hidden"
          : // pointer-events-auto counters the outer Tabs root's
            // pointer-events-none below -- this is the box that actually
            // tracks the visible sheet position (via DrawerContent's own
            // y-transform), so it's the one that should catch touches.
            "pointer-events-auto z-10 flex h-full w-full flex-col overflow-hidden"
      }
    >
      {isDesktop && (
        <DrawerClose
          aria-label="Close drawer"
          className="absolute top-3 right-3 z-20 touch-manipulation max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
          variant="quiet"
          onClick={() => setIsDrawerOpen(false)}
        >
          <XIcon />
        </DrawerClose>
      )}

      {/* Mobile has no true "closed" state -- peek is the floor -- so this
          toggles between minimizing (from half/full) and expanding (from
          peek) rather than just closing. Gives the drag notch and
          tap-open-map collapse (useCollapseDrawerOnMapTap) an explicit,
          discoverable equivalent -- gestures alone aren't discoverable. */}
      {!isDesktop && (
        <DrawerClose
          aria-label={
            snapPoint === "peek" ? "Expand drawer" : "Minimize drawer"
          }
          className="absolute top-3 right-3 z-20 touch-manipulation max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
          variant="quiet"
          onClick={() => setSnapPoint(snapPoint === "peek" ? "half" : "peek")}
        >
          {snapPoint === "peek" ? <ChevronUp /> : <ChevronDown />}
        </DrawerClose>
      )}

      {!isDesktop && (
        <TabList
          aria-label="Content sections"
          className="mt-4 mb-1 flex shrink-0 items-center justify-center gap-1 border-b border-black/10 px-2 dark:border-white/10"
        >
          {destinationTabs.map(({ id, label }) => (
            <Tab
              key={id}
              id={id}
              aria-label={label}
              className="touch-manipulation gap-1.5 px-2 py-3.5"
            >
              <DestinationIcon id={id} size={16} />
              <span className="hidden text-center text-xs font-medium min-[375px]:inline">
                {label}
              </span>
            </Tab>
          ))}
        </TabList>
      )}

      <TabPanels>
        <TabPanel id="explore" className="flex flex-col p-0">
          {!selectedEvents.length && entries.length > 0 && (
            <DrawerHeader className="sticky top-0 z-10 mt-0 w-full bg-background sm:my-2">
              <EventsDrawerHeader
                topMostId={topMostId}
                onSelect={scrollToDate}
              />
            </DrawerHeader>
          )}
          <DrawerBody
            ref={eventsScrollRef}
            className="h-full flex-1 overflow-y-auto p-0"
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
        <TabPanel id="apple" className="flex flex-col">
          <Suspense fallback={null}>
            <AppleMusicDrawerHeader />
            <AppleMusicDrawerBody />
          </Suspense>
        </TabPanel>
      </TabPanels>
    </DrawerContent>
  )

  return (
    <Tabs
      orientation={isDesktop ? "vertical" : "horizontal"}
      // Desktop: unchanged, a normal flex sidebar item. Mobile: an
      // absolutely-positioned overlay (see MapWrapper.tsx, which now
      // renders this inside the map's own relative container) sized to
      // the "full" snap height -- DrawerContent's drag/spring then
      // translates within that fixed box rather than the box itself
      // resizing per snap point. This box itself never moves, so it's
      // pointer-events-none -- without that, its static, transparent
      // "reserved for full height" area would sit on top of the map and
      // swallow every tap/drag even where the sheet has slid away and the
      // map is visibly showing through (peek/half). DrawerContent's own
      // className below opts back into pointer-events-auto, since *that*
      // box is the one whose position actually matches what's on screen.
      className={
        isDesktop
          ? "h-full shrink-0"
          : "pointer-events-none absolute inset-x-0 bottom-0"
      }
      style={isDesktop ? undefined : { height: snapPointsPx[2] }}
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
              <span className="text-center text-[10px] leading-none font-medium">
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
