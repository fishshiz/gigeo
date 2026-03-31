import { EventCard } from "./EventCard"
import { useEvents } from "./components/events-provider"
import { EventDetails } from "./EventDetails"
import { Search } from "./Search"
import { DateRangePicker } from "@workspace/ui/components/ui/DateRangePicker"
import type { Event } from "./lib/types"
import { formatDate } from "./lib/formats"
import { ChevronRight } from "lucide-react"
import { Link } from "@workspace/ui/components/ui/Link"
import { EventFilter } from "./EventFilter"
import { Button } from "@workspace/ui/components/ui/Button"
const Drawer = () => {
  const eventsContext = useEvents()
  const { events, dateRange, setDateRange } = eventsContext
  return (
    <div className="z-10 flex h-full max-h-screen min-h-screen w-full basis-2xl flex-col bg-white shadow-xl sm:top-0 sm:w-md dark:bg-(--color-bg-dark-900)">
      <div className="relative top-0 z-10 flex border-b border-b-slate-200 bg-white px-1 py-2 dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-900)">
        <Search />
        <DateRangePicker
          aria-label="Select timeframe"
          value={dateRange}
          onChange={(date) => date && setDateRange(date)}
        />
      </div>
      <div>
        <EventFilter />
        <Button
          onClick={async () => {
            try {
              // 1. Perform the fetch request
              const response = await fetch("/api/spotify/playlist", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                redirect: "follow",
                body: JSON.stringify({
                  name: "test playlist",
                  description: "playlist created from gigeo app",
                  artists: Object.values(events)
                    .flat()
                    .flatMap((e) => e.attractions?.map((a) => a.name) || []),
                }),
              })
              console.log("Final URL:", response) // Log the final URL for debugging

              if (!response.ok) {
                throw new Error(`Response status: ${response.status}`)
              }

              // 2. Get the final URL after any redirects
              const finalUrl = response.url

              // 3. Open the final URL in a new tab
              // Note: This must be triggered by a direct user action (like a button click handler)
              // to avoid being blocked by the browser's pop-up blocker.
              const newTab = window.open(finalUrl, "_blank")
              if (newTab) {
                newTab.focus() // Focus the newly opened tab
              } else {
                alert(
                  "Pop-up blocked. Please allow pop-ups for this site to open the link in a new tab."
                )
              }
            } catch (error) {
              console.error("Error during fetch:", error)
            }
          }}
        >
          Create Spotify Playlist
        </Button>
        <Button
          onClick={async () => {
            try {
              // 1. Perform the fetch request
              const response = await fetch("/api/spotify/login", {
                redirect: "follow",
              })
              console.log("Final URL:", response) // Log the final URL for debugging

              if (!response.ok) {
                throw new Error(`Response status: ${response.status}`)
              }

              // 2. Get the final URL after any redirects
              const finalUrl = response.url

              // 3. Open the final URL in a new tab
              // Note: This must be triggered by a direct user action (like a button click handler)
              // to avoid being blocked by the browser's pop-up blocker.
              const newTab = window.open(finalUrl, "_blank")
              if (newTab) {
                newTab.focus() // Focus the newly opened tab
              } else {
                alert(
                  "Pop-up blocked. Please allow pop-ups for this site to open the link in a new tab."
                )
              }
            } catch (error) {
              console.error("Error during fetch:", error)
            }
          }}
        >
          Login
        </Button>
      </div>
      {eventsContext.selectedEvent ? (
        <EventDetails eventData={eventsContext.selectedEvent} />
      ) : (
        <EventList events={events} />
      )}
    </div>
  )
}

const EventList = ({ events }: { events: Record<string, Event[]> }) => {
  const entries = Object.entries(events).sort(([dateA], [dateB]) =>
    dateA.localeCompare(dateB)
  )
  const formatDateId = (date: string) =>
    date.toLocaleLowerCase().replace(" ", "")
  return (
    <div className="overflow-y-scroll scroll-smooth">
      {entries.map(([date, events], idx) => (
        <div key={formatDateId(date)}>
          <div
            id={formatDateId(date)}
            className="sticky top-0 z-5 flex scroll-smooth border-b border-b-slate-200 bg-(--color-ivory-700) p-4 dark:border-b-(--color-border-subtle-dark-200) dark:bg-(--color-bg-dark-700) dark:text-(--color-text-primary-dark-700)"
          >
            <h3>{date}</h3>
            {idx < entries.length - 1 && (
              <Link href={`#${formatDateId(entries[idx + 1][0])}`}>
                <ChevronRight />
              </Link>
            )}
          </div>
          <ul className="flex w-full flex-col justify-between gap-4 p-4">
            {events.map((event) => (
              <li className="relative" key={event.id}>
                <EventCard
                  key={event.id}
                  event={event}
                  date={
                    <span className="text-gray-600">
                      {formatDate(event.dates)}
                    </span>
                  }
                />
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  )
}

export { Drawer }
