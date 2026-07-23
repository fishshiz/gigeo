import { MapWrapper } from "./MapWrapper"
import { DrawerWrapper } from "./Drawer/DrawerWrapper"
import { Search } from "./Search"
import { useIsMobile } from "./providers/Breakpoint"
import { AppHeader } from "./Header"
const AppWrapper = () => {
  return (
    <div className="flex h-dvh flex-col overflow-hidden bg-slate-50">
      <AppHeader>
        <Search />
      </AppHeader>
      <div className="flex min-h-0 flex-1 flex-col">
        <MapWrapper />
        {useIsMobile() && <DrawerWrapper />}
      </div>
    </div>
  )
}

export { AppWrapper }
