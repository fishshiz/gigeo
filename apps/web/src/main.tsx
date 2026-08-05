import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { BreakpointProvider } from "./providers/Breakpoint.tsx"
import "@workspace/ui/globals.css"
import { App } from "./App.tsx"
import { ThemeProvider } from "@/components/theme-provider.tsx"
import { EventsProvider } from "./providers/eventsProvider"
import { PlaylistProvider } from "./providers/playlistsProvider.tsx"
import { SearchProvider } from "./providers/searchProvider.tsx"
import { MapViewProvider } from "./providers/mapViewProvider.tsx"
import { DrawerProvider } from "./providers/drawerProvider.tsx"

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <BreakpointProvider>
        <DrawerProvider>
          <SearchProvider>
            <MapViewProvider>
              <EventsProvider>
                <PlaylistProvider>
                  <App />
                </PlaylistProvider>
              </EventsProvider>
            </MapViewProvider>
          </SearchProvider>
        </DrawerProvider>
      </BreakpointProvider>
    </ThemeProvider>
  </StrictMode>
)
