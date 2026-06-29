import { useState } from "react"
import { useSpotifyAuth } from "./hooks/spotify"
import { DialogTrigger, Heading, TooltipTrigger } from "react-aria-components"
import { TagGroup, Tag, TagList } from "react-aria-components"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { Button } from "@workspace/ui/components/ui/Button"
import { Tooltip } from "@workspace/ui/components/ui/Tooltip"
import { Popover } from "@workspace/ui/components/ui/Popover"
import { TextField } from "@workspace/ui/components/ui/TextField"
import {
  Disclosure,
  DisclosurePanel,
} from "@workspace/ui/components/ui/Disclosure"

export function PlaylistButtons() {
  const { status, loading, error, connectSpotify } = useSpotifyAuth()

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
    <div className="absolute right-2 bottom-2 z-10">
      <DialogTrigger>
        <TooltipTrigger>
          <Button aria-label="Filters" variant="secondary" className="relative">
            Manage Spotify
          </Button>
          <Tooltip>Manage Spotify Playlists</Tooltip>
        </TooltipTrigger>
        <Popover showArrow>
          <Dialog className="max-h-[inherit] w-[350px] overflow-auto p-4 outline outline-0">
            <Heading slot="title" className="m-0 mb-2 text-lg font-semibold">
              Playlists
            </Heading>
            <Disclosure
              isExpanded={isExpanded}
              onExpandedChange={setIsExpanded}
            >
              <Heading>
                Create Playlist
                <Button slot="trigger" />
              </Heading>
              <DisclosurePanel>
                <TextField
                  label="Playlist Name"
                  value={name}
                  onChange={(val) => setName(val)}
                />
              </DisclosurePanel>
            </Disclosure>
          </Dialog>
        </Popover>
      </DialogTrigger>
    </div>
  )
}
