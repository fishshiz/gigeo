import { MapWrapper } from "./MapWrapper"
import { Drawer } from "./Drawer"

function App() {
  return (
    <main className="flex h-full">
      <Drawer />
      <MapWrapper />
    </main>
  )
}

export { App }
