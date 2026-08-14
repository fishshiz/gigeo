import { lazy, Suspense } from "react"
import { Search } from "./Search"
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
      </div>
    </div>
  )
}

export { AppWrapper }
