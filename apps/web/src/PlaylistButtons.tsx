import { useState } from "react"
import { useSpotifyAuth } from "./hooks/spotify"
import { Button } from "@workspace/ui/components/ui/Button"
import SpotifyLogo from "./assets/Primary_Logo_Green_CMYK.svg"
import { ReactSVG } from "react-svg"

import { Menu, MenuTrigger, MenuItem } from "@workspace/ui/components/ui/Menu"

export function PlaylistButtons() {
  const { status, loading, error, connectSpotify, getPlaylists } =
    useSpotifyAuth()

  const [name, setName] = useState("My generated playlist")

  let [isExpanded, setIsExpanded] = useState(false)

  if (loading) return <div>Loading auth status…</div>
  if (error) return <div>Auth error: {error}</div>

  if (!status?.logged_in || !status.spotify_connected) {
    return (
      <div>
        <h2>Spotify</h2>
        <p>Your app session is missing or Spotify is not connected yet.</p>
        <button onClick={connectSpotify}>Connect Spotify</button>
      </div>
    )
  }

  return (
    <div className=" ">
      <MenuTrigger>
        <Button aria-label="Filters" variant="secondary" className="relative">
          <ReactSVG className="h-[24px] w-[24px]" src={SpotifyLogo} />
          Connected
        </Button>
        <Menu>
          <MenuItem>Manage Playlists</MenuItem>
          <MenuItem>Logout</MenuItem>
        </Menu>
      </MenuTrigger>
    </div>
  )
}
