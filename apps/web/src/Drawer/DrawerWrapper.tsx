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
import { useEffect, useRef } from "react"
import { VenueDetails } from "../VenueDetails"
import { EventsDrawer, EventsDrawerHeader } from "./EventsDrawer"
import { PlaylistsDrawer } from "./PlaylistsDrawer"
import { useDrawerProvider } from "@/providers/drawerProvider"

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

const DrawerWrapper = () => {
  const { eventsByDate, selectedEvents } = useEventsContext()
  const { isDrawerOpen, setIsDrawerOpen } = useDrawerProvider()
  const eventListRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!eventListRef.current) return
    setIsDrawerOpen(true)
  }, [eventsByDate])
  const isDesktop = useMediaQuery("(min-width: 768px)", {
    defaultValue: false,
    initializeWithValue: false,
  })

  const entries = Object.entries(eventsByDate).sort()

  return (
    <>
      <Tabs orientation="vertical">
        <TabList
          aria-label="Tabs"
          className="border-r border-solid border-l-black"
        >
          <Tab id="explore">
            <Compass size={24} />
          </Tab>
          <Tab id="spotify">
            <ReactSVG className="h-[24px] w-[24px]" src={SpotifyLogo} />
          </Tab>
          <Tab id="apple">
            <ReactSVG
              beforeInjection={(svg) => {
                svg.setAttribute("width", "24")
                svg.setAttribute("height", "24")
                svg.setAttribute(
                  "viewBox",
                  svg.getAttribute("viewBox") || "0 0 24 24"
                )
              }}
              src={AppleMusicLogo}
            />
          </Tab>
        </TabList>
        <TabPanels>
          <TabPanel id="explore" className="p-0">
            <DrawerContent
              isOpen={isDrawerOpen}
              closeDrawer={() => setIsDrawerOpen(false)}
              isBlurred={false}
              notch={isDesktop ? false : true}
              side={isDesktop ? "left" : "bottom"}
              className="z-10 flex h-full w-full max-w-md flex-col overflow-hidden bg-white"
            >
              {!selectedEvents.length && (
                <DrawerHeader className="sticky top-0 z-10 my-2 w-full bg-white">
                  <DrawerClose
                    className="absolute top-4 right-4 z-10"
                    variant="quiet"
                    onClick={() => setIsDrawerOpen(false)}
                  >
                    <XIcon />
                  </DrawerClose>
                  {entries.length > 0 && <EventsDrawerHeader />}
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
            </DrawerContent>
          </TabPanel>

          <TabPanel id="spotify" className="flex items-center justify-center">
            <PlaylistsDrawer />
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
      </Tabs>
    </>
  )
}

export { DrawerWrapper }
