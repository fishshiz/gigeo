import { useSpotifyAuth } from "./hooks/spotify"
import { Button } from "@workspace/ui/components/ui/Button"
import SpotifyLogo from "./assets/Primary_Logo_Green_CMYK.svg"
import { ReactSVG } from "react-svg"
import { CircleCheck, LogOut, TriangleAlert } from "lucide-react"

export function PlaylistButtons() {
  const { status, loading, error, connectSpotify, logout } = useSpotifyAuth()

  if (loading) {
    return (
      <div className="flex items-center gap-2 py-1 text-sm text-muted-foreground">
        <span className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
        Checking Spotify connection…
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300">
        <TriangleAlert size={16} className="shrink-0" />
        Couldn't reach Spotify: {error}
      </div>
    )
  }

  if (!status?.logged_in || !status.spotify_connected) {
    return (
      <div className="flex items-center justify-between gap-3 rounded-lg border border-black/10 bg-white/80 p-3 dark:border-white/10 dark:bg-zinc-900/70">
        <div className="min-w-0">
          <p className="text-sm font-semibold text-primary">
            Connect Spotify
          </p>
          <p className="text-xs text-muted-foreground">
            Link your account to create and manage playlists.
          </p>
        </div>
        <Button
          onClick={connectSpotify}
          className="shrink-0 gap-2 bg-[#1DB954] text-white hover:bg-[#1ed760]"
        >
          <ReactSVG className="h-4 w-4 [&_svg_path]:fill-white" src={SpotifyLogo} />
          Connect
        </Button>
      </div>
    )
  }

  return (
    <div className="flex items-center justify-between gap-3">
      <span className="inline-flex items-center gap-1.5 rounded-full bg-[#1DB954]/10 px-2.5 py-1 text-xs font-medium text-[#1a9c48] dark:bg-[#1DB954]/15 dark:text-[#3ddb70]">
        <CircleCheck size={14} />
        Spotify connected
      </span>
      <Button
        onClick={logout}
        variant="quiet"
        aria-label="Log out of Spotify"
        className="gap-1.5 text-muted-foreground"
      >
        <LogOut size={14} />
        Logout
      </Button>
    </div>
  )
}
