import { Sparkles } from "lucide-react"
import { Button } from "@workspace/ui/components/ui/Button"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { Popover } from "@workspace/ui/components/ui/Popover"
import { DialogTrigger } from "react-aria-components"

/**
 * Neutral, provider-agnostic badge shown on events matching one of the
 * viewer's Spotify top artists, directly or via Apple Music's
 * similar-artists expansion. Tap-to-reveal (not hover) on both desktop and
 * mobile, since this drawer UI has no other hover-dependent affordance.
 * Stops propagation so tapping it doesn't also open the event's details.
 */
export function ForYouTag({
  matchedArtist,
  matchedVia,
}: {
  matchedArtist: string
  matchedVia?: string | null
}) {
  return (
    <div
      onClick={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
    >
      <DialogTrigger>
        <Button className="flex !h-auto w-16 min-w-fit shrink-0 cursor-default items-center gap-1 rounded-full border border-neutral-200 bg-(--accent-bg) px-6 py-0.5 text-xs font-medium whitespace-nowrap text-(--text-on-accent) transition hover:border-neutral-300 dark:border-neutral-600 dark:bg-neutral-900 dark:text-neutral-300 dark:hover:border-neutral-500">
          <Sparkles aria-hidden className="h-3 w-3 shrink-0" />
          <span>For You</span>
        </Button>
        <Popover showArrow placement="top">
          <Dialog className="w-auto max-w-[220px] p-3 text-sm">
            {matchedVia ? (
              <>
                Similar to <span className="font-semibold">{matchedVia}</span>,
                who you listen to
              </>
            ) : (
              <>
                Because you've been listening to{" "}
                <span className="font-semibold">{matchedArtist}</span>
              </>
            )}
          </Dialog>
        </Popover>
      </DialogTrigger>
    </div>
  )
}
