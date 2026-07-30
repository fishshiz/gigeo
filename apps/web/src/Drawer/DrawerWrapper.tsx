import { EventDetails } from "../EventDetails"
import { XIcon } from "lucide-react"
import {
  DrawerBody,
  DrawerContent,
  DrawerHeader,
  DrawerClose,
} from "@workspace/ui/components/ui/Drawer"
import { useEventsContext } from "../providers/eventsProvider"
import { useMediaQuery } from "usehooks-ts"
import { useEffect, useRef, useState, lazy, Suspense } from "react"
import { VenueDetails } from "../VenueDetails"
import { EventsDrawer, EventsDrawerHeader } from "./EventsDrawer"
import type { Key } from "react-aria-components/Tabs"
import { useDrawerProvider } from "@/providers/drawerProvider"

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

  const [activeTab, setActiveTab] = useState<Key>("explore")
  const eventListRef = useRef<HTMLDivElement>(null)

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
    setIsDrawerOpen(true)
    setActiveTab("explore")
  }, [eventsByDate, setIsDrawerOpen])
  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })

  const entries = Object.entries(eventsByDate).sort()

  return (
    <Tabs
      orientation={isDesktop ? "vertical" : "horizontal"}
      className="h-[40vh] shrink-0 md:h-full md:w-[30rem]"
      selectedKey={activeTab}
      onSelectionChange={(t) => setActiveTab(t)}
    >
      {isDesktop && (
        <TabList
          aria-label="Content sections"
          className="flex-col items-center gap-1 border-r border-black/10 py-2 dark:border-white/10"
        >
          {destinationTabs.map(({ id, label }) => (
            <Tab
              key={id}
              id={id}
              aria-label={label}
              className="w-16 flex-col gap-1 rounded-lg px-2 py-2"
            >
              <DestinationIcon id={id} size={20} />
              <span className="text-[10px] leading-none font-medium">
                {label}
              </span>
            </Tab>
          ))}
        </TabList>
      )}

      <DrawerContent
        isOpen={isDrawerOpen}
        closeDrawer={() => setIsDrawerOpen(false)}
        notch={isDesktop ? false : true}
        side={isDesktop ? "left" : "bottom"}
        className="z-10 flex h-full w-full flex-col overflow-hidden"
      >
        <DrawerClose
          className="absolute top-3 right-3 z-20"
          variant="quiet"
          onClick={() => setIsDrawerOpen(false)}
        >
          <XIcon />
        </DrawerClose>

        {!isDesktop && (
          <TabList
            aria-label="Content sections"
            className="flex shrink-0 items-center justify-center gap-1.5 border-b border-black/10 px-3 pt-1 pr-14 pb-2 dark:border-white/10"
          >
            {destinationTabs.map(({ id, label }) => (
              <Tab key={id} id={id} className="gap-1.5 px-3">
                <DestinationIcon id={id} size={16} />
                <span className="text-xs font-medium">{label}</span>
              </Tab>
            ))}
          </TabList>
        )}

        <TabPanels>
          <TabPanel id="explore" className="p-0">
            {!selectedEvents.length && entries.length > 0 && (
              <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-background">
                <EventsDrawerHeader />
              </DrawerHeader>
            )}
            <DrawerBody className="h-full flex-1 overflow-y-auto scroll-smooth">
              {selectedEvents.length === 1 ? (
                <EventDetails eventData={selectedEvents[0]} />
              ) : selectedEvents.length ? (
                <VenueDetails events={selectedEvents} />
              ) : (
                <EventsDrawer />
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
    </Tabs>
  )
}

export { DrawerWrapper }
