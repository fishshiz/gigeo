import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { BreakpointProvider } from "./providers/Breakpoint.tsx"
import "@workspace/ui/globals.css"
import { App } from "./App.tsx"
import { ThemeProvider } from "@/components/theme-provider.tsx"
import { EventsProvider as OldProvider } from "./components/events-provider.tsx"
import { EventsProvider } from "./providers/eventsProvider"

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <BreakpointProvider>
        <OldProvider>
          <EventsProvider>
            <App />
          </EventsProvider>
        </OldProvider>
      </BreakpointProvider>
    </ThemeProvider>
  </StrictMode>
)
