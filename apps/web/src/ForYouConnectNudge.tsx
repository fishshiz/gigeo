import { useState } from "react"
import { X } from "lucide-react"
import { Button } from "@workspace/ui/components/ui/Button"
import SpotifyLogo from "./assets/Primary_Logo_Green_CMYK.svg"
import { ReactSVG } from "react-svg"
import { useSpotifyAuth } from "./hooks/spotify"

const DISMISSED_STORAGE_KEY = "gigeo:for-you-nudge-dismissed"

function readDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISSED_STORAGE_KEY) === "true"
  } catch {
    // localStorage unavailable (e.g. private browsing) — show the nudge.
    return false
  }
}

function persistDismissed() {
  try {
    localStorage.setItem(DISMISSED_STORAGE_KEY, "true")
  } catch {
    // Ignore — localStorage may be unavailable (e.g. private browsing).
  }
}

/**
 * One-time, dismissible prompt surfacing the "for you" feature to users who
 * haven't connected Spotify yet — the only way this gets discovered
 * organically, since matched events are otherwise invisible until connected.
 * Hides permanently once dismissed or once Spotify is connected.
 */
export function ForYouConnectNudge() {
  const { status, loading, connectSpotify } = useSpotifyAuth()
  const [dismissed, setDismissed] = useState(readDismissed)

  if (dismissed || loading || status?.spotify_connected) {
    return null
  }

  const dismiss = () => {
    persistDismissed()
    setDismissed(true)
  }

  return (
    <div className="relative mb-2 flex items-center justify-between gap-3 rounded-lg border border-black/10 bg-white/80 p-3 pr-8 dark:border-white/10 dark:bg-zinc-900/70">
      <div className="min-w-0">
        <p className="text-sm font-semibold text-primary">
          See shows for artists you follow
        </p>
        <p className="text-xs text-muted-foreground">
          Connect Spotify to tag events matching your top artists.
        </p>
      </div>
      <Button
        onClick={connectSpotify}
        className="shrink-0 gap-2 bg-[#1DB954] text-white hover:bg-[#1ed760]"
      >
        <ReactSVG className="h-4 w-4 [&_svg_path]:fill-white" src={SpotifyLogo} />
        Connect
      </Button>
      <button
        type="button"
        aria-label="Dismiss"
        onClick={dismiss}
        className="absolute top-2 right-2 rounded p-0.5 text-muted-foreground hover:text-foreground"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  )
}
