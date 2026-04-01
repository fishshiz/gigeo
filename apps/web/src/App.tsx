import { MapWrapper } from "./MapWrapper"
import { Drawer } from "./Drawer"
import { Search } from "./Search"
import { PlaylistButtons } from "./PlaylistButtons"
function App() {
  return (
    <main className="sm:flex">
      <Drawer />
      <Search />
      <PlaylistButtons />
      <MapWrapper />
    </main>
  )
}

export { App }
