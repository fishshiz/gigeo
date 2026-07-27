import { lazy, Suspense } from "react"
import { DrawerWrapper } from "./Drawer/DrawerWrapper"
import { Search } from "./Search"
import { useIsMobile } from "./providers/Breakpoint"
import { AppHeader } from "./Header"

const MapWrapper = lazy(() =>
  import("./MapWrapper").then((m) => ({ default: m.MapWrapper }))
)

const AppWrapper = () => {
  return (
    <div className="flex h-dvh flex-col overflow-hidden bg-slate-50">
      <AppHeader>
        <Search />
      </AppHeader>
      <div className="flex min-h-0 flex-1 flex-col">
        <Suspense fallback={null}>
          <MapWrapper />
        </Suspense>
        {useIsMobile() && <DrawerWrapper />}
      </div>
    </div>
  )
}

export { AppWrapper }
