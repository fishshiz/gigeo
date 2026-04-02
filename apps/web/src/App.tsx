import { MapWrapper } from "./MapWrapper"
import { DrawerWrapper } from "./Drawer"
import { Search } from "./Search"
import { PlaylistButtons } from "./PlaylistButtons"
function App() {
  return (
    <main className="sm:flex">
      <DrawerWrapper />
      <Search />
      <PlaylistButtons />
      <MapWrapper />
    </main>
  )
}

export { App }
