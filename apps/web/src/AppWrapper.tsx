import { useState } from "react"
import { MapWrapper } from "./MapWrapper"
import { DrawerWrapper } from "./Drawer"
import { Search } from "./Search"
import { PlaylistButtons } from "./PlaylistButtons"
import { ChevronRightIcon } from "lucide-react"
import { useIsMobile } from "./providers/Breakpoint"
import { DrawerTrigger } from "@workspace/ui/components/ui/Drawer"
import { AppHeader } from "./Header"
const AppWrapper = () => {
  const [drawerOpen, setDrawerOpen] = useState(false)

  return (
    <div className="flex h-dvh flex-col">
      <AppHeader>
        <Search />

        <PlaylistButtons />
      </AppHeader>
      <div className="h-full">
        <MapWrapper />
      </div>

      {useIsMobile() && (
        <DrawerWrapper
          drawerOpen={drawerOpen}
          setDrawerOpen={(isOpen) => {
            setDrawerOpen(isOpen)
          }}
        />
      )}
    </div>
  )
}

export { AppWrapper }
