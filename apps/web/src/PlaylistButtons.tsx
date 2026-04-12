import { Button } from "@workspace/ui/components/ui/Button"
import { MenuTrigger, Menu, MenuItem } from "@workspace/ui/components/ui/Menu"
import SpotifyLogo from "@/assets/Primary_Logo_Green_CMYK.svg"
import { ReactSVG } from "react-svg"
import { useEventsContext } from "./providers/eventsProvider"

const PlaylistButtons = () => {
  const { eventsByDate } = useEventsContext()
  const loginSpotify = async () => {
    try {
      // 1. Perform the fetch request
      const response = await fetch("/api/spotify/login", {
        redirect: "follow",
        mode: "no-cors",
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
  }
  const createSpotifyPlaylist = async () => {
    try {
      const artists = Object.values(eventsByDate)
        .flat()
        .filter((e) => {
          const { classifications } = e
          if (classifications) {
            return classifications[0].segment?.name === "Music"
          }
          return false
        })
        .flatMap((e) => e.attractions?.map((a) => a.name) || [])
      // 1. Perform the fetch request
      const response = await fetch("/api/spotify/playlist", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        redirect: "follow",
        body: JSON.stringify({
          name: "test playlist",
          description: "playlist created from gigeo app",
          artists: artists,
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
  }
  return (
    <div className="absolute top-0 left-0 z-15 m-2">
      <MenuTrigger>
        <Button>
          <ReactSVG className="h-4 w-4" src={SpotifyLogo} />
        </Button>
        <Menu>
          <MenuItem onAction={() => loginSpotify()}>Login</MenuItem>
          <MenuItem onAction={() => createSpotifyPlaylist()}>
            Create Playlist
          </MenuItem>
        </Menu>
      </MenuTrigger>
    </div>
  )
}

export { PlaylistButtons }
