import { Button } from "@workspace/ui/components/ui/Button"
import { useEvents } from "./components/events-provider"

const PlaylistButtons = () => {
  const { events } = useEvents()
  return (
    <div className="absolute">
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
  )
}

export { PlaylistButtons }
