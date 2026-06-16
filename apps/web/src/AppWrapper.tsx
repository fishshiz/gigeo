import { useState } from "react"
import { MapWrapper } from "./MapWrapper"
import { DrawerWrapper } from "./Drawer"
import { Search } from "./Search"
import { PlaylistButtons } from "./PlaylistButtons"
import { ChevronRightIcon } from "lucide-react"
import { useIsMobile } from "./providers/Breakpoint"
import { DrawerTrigger } from "@workspace/ui/components/ui/Drawer"
const AppWrapper = () => {
  const [drawerOpen, setDrawerOpen] = useState(false)

  return (
    <>
      {!useIsMobile() && (
        <DrawerWrapper
          drawerOpen={drawerOpen}
          setDrawerOpen={(isOpen) => {
            setDrawerOpen(isOpen)
          }}
        />
      )}
      <div className="relative h-full w-full">
        <Search />
        <PlaylistButtons />
        <MapWrapper />
        {!drawerOpen && (
          <DrawerTrigger
            onClick={() => setDrawerOpen(true)}
            className="shadow:lg absolute bottom-0 z-20 flex h-32 w-full items-center justify-center border-t border-gray-300 bg-white p-0 p-4 text-sm font-medium text-gray-700 sm:top-1/2 sm:left-0 sm:h-min sm:w-min sm:-translate-y-1/2 sm:transform sm:rounded-tr-lg sm:rounded-br-lg"
          >
            <ChevronRightIcon className="stroke-gray-700" />
          </DrawerTrigger>
        )}
      </div>
      {useIsMobile() && (
        <DrawerWrapper
          drawerOpen={drawerOpen}
          setDrawerOpen={(isOpen) => {
            setDrawerOpen(isOpen)
          }}
        />
      )}
    </>
  )
}

export { AppWrapper }
