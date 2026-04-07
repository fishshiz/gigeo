import { useState } from "react"
import { MapWrapper } from "./MapWrapper"
import { DrawerWrapper } from "./Drawer"
import { Search } from "./Search"
import { PlaylistButtons } from "./PlaylistButtons"
import { ChevronUpIcon } from "lucide-react"
import { useIsMobile } from "./providers/Breakpoint"
import { DrawerTrigger } from "@workspace/ui/components/ui/Drawer"
const AppWrapper = () => {
  const [drawerOpen, setDrawerOpen] = useState(true)

  return (
    <>
      {!useIsMobile() && (
        <DrawerWrapper
          drawerOpen={drawerOpen}
          setDrawerOpen={(isOpen) => {
            console.log("open", isOpen)
            setDrawerOpen(isOpen)
          }}
        />
      )}
      <div className="relative h-full w-full">
        <Search />
        <PlaylistButtons />
        <MapWrapper drawerOpen={drawerOpen} />
        {!drawerOpen && (
          <div className="absolute">
            <DrawerTrigger
              onClick={() => setDrawerOpen(true)}
              className="shadow:lg absolute bottom-100 z-20 flex w-full items-center justify-center border-t border-gray-300 bg-white p-0 p-4 text-sm font-medium text-gray-700 sm:relative sm:left-0 sm:w-auto"
            >
              <ChevronUpIcon className="stroke-gray-700" />
              <span>Tap to open drawer :)</span>
            </DrawerTrigger>
          </div>
        )}
      </div>
      {useIsMobile() && (
        <DrawerWrapper
          drawerOpen={drawerOpen}
          setDrawerOpen={(isOpen) => {
            console.log("open", isOpen)
            setDrawerOpen(isOpen)
          }}
        />
      )}
    </>
  )
}

export { AppWrapper }
