import { useAppleMusicAuth } from "./hooks/appleMusic"
import { Button } from "@workspace/ui/components/ui/Button"
import AppleMusicLogo from "./assets/Apple_Music_Icon_RGB_lg_073120.svg"
import { ReactSVG } from "react-svg"
import { CircleCheck, LogOut, TriangleAlert } from "lucide-react"

export function AppleMusicButtons() {
  const { status, loading, error, connectAppleMusic, logout } =
    useAppleMusicAuth()

  if (loading) {
    return (
      <div className="flex items-center gap-2 py-1 text-sm text-muted-foreground">
        <span className="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
        Checking Apple Music connection…
      </div>
    )
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 rounded-lg border border-red-200 bg-red-50 p-3 text-sm text-red-700 dark:border-red-900 dark:bg-red-950 dark:text-red-300">
        <TriangleAlert size={16} className="shrink-0" />
        Couldn't reach Apple Music: {error}
      </div>
    )
  }

  if (!status?.logged_in || !status.apple_music_connected) {
    return (
      <div className="flex items-center justify-between gap-3 rounded-lg border border-black/10 bg-white/80 p-3 dark:border-white/10 dark:bg-zinc-900/70">
        <div className="min-w-0">
          <p className="text-sm font-semibold text-primary">
            Connect Apple Music
          </p>
          <p className="text-xs text-muted-foreground">
            Link your account to create and manage playlists.
          </p>
        </div>
        <Button
          onClick={connectAppleMusic}
          className="shrink-0 gap-2 bg-black text-white hover:bg-neutral-800 dark:bg-white dark:text-black dark:hover:bg-neutral-200"
        >
          <ReactSVG
            className="h-4 w-4 shrink-0 [&_svg_path]:fill-white dark:[&_svg_path]:fill-black"
            style={{ width: 16, height: 16 }}
            beforeInjection={(svg) => {
              // The raw asset's intrinsic size is much larger than 16px —
              // Tailwind's h-4/w-4 alone doesn't constrain an injected raw
              // <svg>, same issue already worked around for this exact
              // file in DrawerWrapper.tsx's DestinationIcon.
              svg.setAttribute("width", "16")
              svg.setAttribute("height", "16")
              svg.setAttribute("viewBox", svg.getAttribute("viewBox") || "0 0 24 24")
            }}
            src={AppleMusicLogo}
          />
          Connect
        </Button>
      </div>
    )
  }

  return (
    <div className="flex items-center justify-between gap-3">
      <span className="inline-flex items-center gap-1.5 rounded-full bg-black/10 px-2.5 py-1 text-xs font-medium text-black dark:bg-white/15 dark:text-white">
        <CircleCheck size={14} />
        Apple Music connected
      </span>
      <Button
        onClick={logout}
        variant="quiet"
        aria-label="Log out of Apple Music"
        className="gap-1.5 text-muted-foreground"
      >
        <LogOut size={14} />
        Logout
      </Button>
    </div>
  )
}
