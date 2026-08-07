import type { AmArtistFull, ExternalLinks } from "./lib/types"
import { Button } from "@workspace/ui/components/ui/Button"
import { Link } from "@workspace/ui/components/ui/Link"
import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { useCallback, useEffect, useState, useRef } from "react"
import { ResponsiveImage } from "@workspace/ui/components/ui/ResponsiveImage"
import {
  ArrowLeftIcon,
  GlobeIcon,
  MapPinIcon,
  DollarSignIcon,
  ClockIcon,
  MusicIcon,
} from "lucide-react"
import WikiLogo from "@/assets/wikipedia-w-brands-solid-full.svg"
import IgLogo from "@/assets/instagram.svg"
import { ReactSVG } from "react-svg"
import "react-social-icons/instagram"
import type { EventResponse } from "./hooks/eventsStream"
import { useEventsContext } from "./providers/eventsProvider"
import { buildArtworkUrl, normalizeBg } from "./lib/artwork"
import { formatTime } from "./lib/dates"
import { ticketmasterAttractionIds } from "./lib/performers"

/** PredictHQ never provides a ticket purchase link -- when `url` is set on
 * a PredictHQ-sourced event, it's the backend's Apple Music artist-page
 * backfill instead (see `predicthq::artwork::backfill_artwork`). Labeling
 * that link "Tickets" would wrongly suggest it's a place to buy tickets. */
const eventLinkLabel = (event: Pick<EventResponse, "source">) =>
  event.source === "predicthq" ? "Listen" : "Tickets"

const formatPriceRange = (range: { currency: string; min: number; max: number }) => {
  const formatter = new Intl.NumberFormat(undefined, {
    style: "currency",
    currency: range.currency,
  })
  return range.min === range.max
    ? formatter.format(range.min)
    : `${formatter.format(range.min)} - ${formatter.format(range.max)}`
}

/** Per-performer state for the `/api/future-events` lookup -- distinct
 * from "loaded with zero events" so the UI can tell "still fetching" and
 * "the request failed" apart from a genuinely empty result. */
type FutureEventsState =
  | { status: "loading" }
  | { status: "error" }
  | { status: "loaded"; events: EventResponse[] }

const NO_FUTURE_EVENTS: FutureEventsState = { status: "loaded", events: [] }

const EventDetails = ({
  eventData,
  otherShowtimes = [],
}: {
  eventData: EventResponse
  /** Other showtimes of this same event today (same name/venue/day),
   * excluding eventData itself. */
  otherShowtimes?: EventResponse[]
}) => {
  const { performers } = eventData
  const [futureEvents, setFutureEvents] = useState<
    Record<string, FutureEventsState>
  >({})
  const [showStickyHeader, setShowStickyHeader] = useState(false)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const isMountedRef = useRef(true)
  const { selectEvents } = useEventsContext()
  const shouldReduceMotion = useReducedMotion()

  useEffect(() => {
    // Explicitly re-arm on setup (not just the useRef(true) initializer) --
    // StrictMode's dev-mode setup/cleanup/setup dance would otherwise leave
    // this stuck at false after the simulated unmount, silently dropping
    // every future-events response for the rest of the component's life.
    isMountedRef.current = true
    return () => {
      isMountedRef.current = false
    }
  }, [])

  const fetchFutureEvents = useCallback((id: string) => {
    setFutureEvents((prev) => ({ ...prev, [id]: { status: "loading" } }))
    fetch(`/api/future-events?id=${id}`)
      .then((res) => {
        if (!res.ok) throw new Error("Request failed")
        return res.json()
      })
      .then((events: EventResponse[]) => {
        if (!isMountedRef.current) return
        setFutureEvents((prev) => ({ ...prev, [id]: { status: "loaded", events } }))
      })
      .catch((e) => {
        console.error("Failed to fetch future events", e)
        if (!isMountedRef.current) return
        setFutureEvents((prev) => ({ ...prev, [id]: { status: "error" } }))
      })
  }, [])

  useEffect(() => {
    ticketmasterAttractionIds(eventData).forEach(fetchFutureEvents)
  }, [eventData, fetchFutureEvents])

  const futureEventsFor = (id: string | null | undefined): FutureEventsState =>
    (id && futureEvents[id]) || NO_FUTURE_EVENTS

  // Performers already matched to a canonical artist -- attached
  // backend-side (see apps/api/src/artists/lookup.rs) rather than fetched
  // on demand from here, as this used to. The type predicate narrows
  // `enrichment` to non-null for every consumer below, rather than each
  // one needing its own `!` assertion.
  const enrichedPerformers = (performers ?? []).filter(
    (performer): performer is typeof performer & { enrichment: AmArtistFull } =>
      Boolean(performer.enrichment)
  )

  // scroll handler for the pane -- this component's own root never
  // overflows itself (it's exactly as tall as its content), so the pane
  // that actually scrolls is our parent (DrawerBody in production, see
  // Drawer/DrawerWrapper.tsx). Watch that, not scrollRef itself.
  useEffect(() => {
    const scrollParent = scrollRef.current?.parentElement
    if (!scrollParent) return

    const handleScroll = () => {
      // tweak threshold to taste (depends on hero height)
      const threshold = 160
      setShowStickyHeader(scrollParent.scrollTop > threshold)
    }

    handleScroll()
    scrollParent.addEventListener("scroll", handleScroll)
    return () => scrollParent.removeEventListener("scroll", handleScroll)
  }, [])

  return (
    // overflow-y-scroll removed: this div never actually overflows itself
    // (see the scroll-listener comment above -- its content is always
    // exactly as tall as it is). Worse than just dead, it was actively
    // harmful: any overflow value other than visible makes an element a
    // scroll container, and position:sticky sticks relative to its
    // nearest scroll-container ancestor -- so the sticky header below was
    // sticking relative to *this* div (which never scrolls) instead of
    // the real scrolling parent, meaning it never actually pinned to the
    // viewport, just scrolled away like normal content. Caught via
    // getBoundingClientRect() during animation verification, not visible
    // from a colors-only check.
    <div ref={scrollRef} className="relative">
      {/* sticky compact header inside the pane */}
      <AnimatePresence>
        {showStickyHeader && (
          <motion.div
            key="sticky-header"
            initial={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
            className="sticky top-0 z-20 flex items-center justify-between gap-2 border-b border-slate-800/40 bg-slate-950/90 py-2 pe-14 ps-3 text-xs text-slate-100 backdrop-blur"
          >
            <div className="flex min-w-0 items-center">
              {/* bg-(--surface-secondary) is repeated under dark: because Button's
                  own "secondary" variant carries its own dark:bg-neutral-700 --
                  without the explicit dark: pair here, that wins the cascade over
                  the unprefixed override (confirmed empirically, not assumed). */}
              <Button
                aria-label="Back to events"
                className="z-10 touch-manipulation bg-(--surface-secondary) dark:border-(--color-border-subtle-dark-200) dark:bg-(--surface-secondary) max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
                variant="secondary"
                onClick={() => selectEvents([])}
              >
                <ArrowLeftIcon aria-hidden className="h-4 w-4 rtl:-scale-x-100" />
              </Button>
              <div className="ms-2 flex min-w-0 flex-col">
                <div className="truncate font-semibold">{eventData.name}</div>
                <div className="flex gap-2 text-[11px] text-slate-300">
                  <span dir="ltr" className="truncate">
                    {eventData.datesPretty}
                  </span>
                  <span className="truncate">· {eventData.venue?.name ?? "—"}</span>
                </div>
              </div>
            </div>

            {eventData.url && (
              <Link
                href={eventData.url}
                target="_blank"
                variant="button"
                className="ms-2 shrink-0 touch-manipulation rounded-full bg-(--accent-bg) px-2 py-1 text-[11px] font-medium text-(--text-on-accent) no-underline max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
              >
                {eventLinkLabel(eventData)}
              </Link>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      {/* back + top-right tickets over hero -- hidden once the sticky
          header takes over the same two controls, so there's never a
          duplicate "Back to events" / tickets link in the tab order.
          initial={false} on both AnimatePresence wrappers: these are
          visible by default on first paint, so that first render must
          not animate in -- only later toggles (scrolling past/back
          above the threshold) should. */}
      <AnimatePresence initial={false}>
        {!showStickyHeader && (
          <motion.div
            key="hero-back"
            initial={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
          >
            <Button
              aria-label="Back to events"
              className="absolute top-2 start-2 z-10 touch-manipulation bg-(--surface-secondary) dark:border-(--color-border-subtle-dark-200) dark:bg-(--surface-secondary) max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
              variant="secondary"
              onClick={() => selectEvents([])}
            >
              <ArrowLeftIcon aria-hidden className="h-4 w-4 rtl:-scale-x-100" />
            </Button>
          </motion.div>
        )}
      </AnimatePresence>

      <AnimatePresence initial={false}>
        {!showStickyHeader && eventData.url && (
          <motion.div
            key="hero-tickets"
            initial={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
          >
            <Link
              variant="button"
              className="absolute top-2 end-14 z-10 touch-manipulation bg-(--accent-bg) text-(--text-on-accent) no-underline max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
              href={eventData.url}
              target="_blank"
            >
              {eventLinkLabel(eventData)}
            </Link>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="relative">
        <div className="absolute top-0 start-0 z-[1] h-full w-full bg-gradient-to-t from-(--surface-scrim) to-transparent opacity-85" />
        <ResponsiveImage
          sources={eventData.images}
          alt={eventData.name}
          loading="eager"
        />
        <h3 className="absolute bottom-2 start-2 end-2 z-[2] line-clamp-2 text-2xl font-semibold text-(--text-on-scrim)">
          {eventData.name}
        </h3>
      </div>

      {/* rest of your content exactly as before */}
      <div className="p-2">
        <div className="flex flex-col justify-between">
          <div className="flex items-center gap-1">
            <ClockIcon aria-hidden className="h-4 w-4 shrink-0" />
            <span dir="ltr" className="min-w-0 truncate">
              {eventData.datesPretty}
            </span>
          </div>
          <div className="flex items-center gap-1">
            <MapPinIcon aria-hidden className="h-4 w-4 shrink-0" />
            <span className="min-w-0 truncate">
              {eventData.venue?.name ?? "—"}
            </span>
          </div>
          {eventData.priceRanges && (
            <div className="flex items-center gap-1">
              <DollarSignIcon aria-hidden className="h-4 w-4 shrink-0" />
              <span dir="ltr" className="tabular-nums">
                {formatPriceRange(eventData.priceRanges[0])}
              </span>
            </div>
          )}
        </div>
      </div>

      {otherShowtimes.length > 0 && (
        <div className="p-2">
          <h3 className="text-xs tracking-wide text-slate-400 uppercase">
            Other showtimes today
          </h3>
          <ul className="mt-1 flex flex-col gap-2 text-sm">
            {otherShowtimes.map((showtime) => (
              <li
                key={showtime.id}
                className="flex items-center justify-between gap-2 border-b border-slate-700/50 pb-2 last:border-b-0"
              >
                <span dir="ltr">
                  {(showtime.dates && formatTime(showtime.dates)) ||
                    showtime.datesPretty}
                </span>
                {showtime.url && (
                  // text-(--text-link) is repeated under dark: because Link's
                  // default "primary" variant carries its own dark:text-blue-500
                  // -- without the explicit dark: pair, that wins the cascade
                  // over the unprefixed override (confirmed empirically).
                  <Link
                    href={showtime.url}
                    target="_blank"
                    className="text-(--text-link) no-underline dark:text-(--text-link)"
                  >
                    {eventLinkLabel(showtime)}
                  </Link>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      {enrichedPerformers.length
        ? enrichedPerformers.map((performer) => {
            const performerId = performer.id
            return (
              <ArtistCard
                key={performer.enrichment.id || performerId || performer.name}
                artist={performer.enrichment}
                similarArtists={performer.enrichment.similar_artists}
                externalLinks={performer.externalLinks ?? undefined}
                futureEvents={futureEventsFor(performerId)}
                onRetryFutureEvents={
                  performerId ? () => fetchFutureEvents(performerId) : undefined
                }
              />
            )
          })
        : performers?.map((performer) => {
            if (!performer.id) return null
            const id = performer.id
            return (
              <div key={id}>
                <h4 className="text-lg font-semibold">{performer.name}</h4>
                <UpcomingEvents
                  {...futureEventsFor(id)}
                  onRetry={() => fetchFutureEvents(id)}
                />
              </div>
            )
          })}
    </div>
  )
}

type SimilarArtist = {
  id: string
  name: string
}

type ArtistCardProps = {
  artist: AmArtistFull
  similarArtists?: SimilarArtist[]
  artworkSize?: number
  externalLinks?: ExternalLinks
  futureEvents?: FutureEventsState
  onRetryFutureEvents?: () => void
}

export const ArtistCard: React.FC<ArtistCardProps> = ({
  artist,
  similarArtists = [],
  externalLinks = {},
  futureEvents = NO_FUTURE_EVENTS,
  onRetryFutureEvents,
  artworkSize = 200,
}) => {
  const { name, genres = [], artwork, apple_music_url } = artist

  // A canonical artist matched via Spotify (rather than Apple Music) can
  // legitimately have no artwork at all -- render the card without an
  // image rather than crashing on a missing url.
  const imgUrl = artwork ? buildArtworkUrl(artwork, artworkSize) : undefined
  const bgColor = normalizeBg(artwork?.bgColor)
  const primaryGenre = genres[0]
  const wikiUrl = externalLinks.wiki?.[0]?.url
  const homepageUrl = externalLinks.homepage?.[0]?.url
  const instagramUrl = externalLinks.instagram?.[0]?.url

  return (
    <div
      className="grid gap-4 p-4 text-slate-50 shadow-lg dark:text-(--color-text-secondary-600)"
      style={{ backgroundColor: bgColor }}
    >
      {/* Artwork block with bgColor */}
      <div className="relative flex shrink-0 gap-4">
        {imgUrl && (
          <div
            className="overflow-hidden rounded-2xl"
            style={{
              width: artworkSize,
              height: artworkSize,
              backgroundColor: bgColor,
            }}
          >
            <ResponsiveImage
              sources={[
                {
                  url: imgUrl,
                  width: artworkSize,
                  height: artworkSize,
                  ratio: "1:1",
                  fallback: true,
                },
              ]}
              alt={`${name} artwork`}
            />
          </div>
        )}
        <div>
          <h2 className="truncate text-xl font-semibold">{name}</h2>
          {primaryGenre && (
            <p className="mt-1 text-sm text-slate-300">{primaryGenre}</p>
          )}
          <ul className="flex flex-col">
            {wikiUrl && <ExternalLink url={wikiUrl} label="Wikipedia" />}
            {homepageUrl && <ExternalLink url={homepageUrl} label="Website" />}
            {instagramUrl && (
              <ExternalLink url={instagramUrl} label="Instagram" />
            )}
            {apple_music_url && (
              <ExternalLink url={apple_music_url} label="Apple Music" />
            )}
          </ul>
        </div>
      </div>

      {/* Text / metadata */}
      <div className="flex flex-1 flex-col justify-between">
        {similarArtists.length > 0 && (
          <div className="mt-4">
            <h3 className="text-xs tracking-wide text-slate-400 uppercase">
              Similar artists
            </h3>
            <ul className="mt-1 flex flex-wrap gap-2 text-sm">
              {similarArtists.slice(0, 6).map((a) => (
                <li
                  key={a.id}
                  className="max-w-[10rem] truncate rounded-full bg-slate-800/70 px-3 py-1"
                >
                  {a.name}
                </li>
              ))}
            </ul>
          </div>
        )}

        <UpcomingEvents {...futureEvents} onRetry={onRetryFutureEvents} />
      </div>
    </div>
  )
}

const ExternalLink = ({ url, label }: { url: string; label: string }) => {
  return (
    <li className="flex">
      <Link
        href={url}
        target="_blank"
        className="my-1 flex items-center fill-(--text-link) text-(--text-link) no-underline dark:fill-(--text-link) dark:text-(--text-link)"
      >
        {label === "Wikipedia" ? (
          <ReactSVG
            className="h-[24px] w-[24px]"
            src={WikiLogo}
            beforeInjection={(svg) => svg.setAttribute("aria-hidden", "true")}
          />
        ) : label === "Instagram" ? (
          <ReactSVG
            className="me-[4px] h-[24px] w-[24px]"
            src={IgLogo}
            beforeInjection={(svg) => svg.setAttribute("aria-hidden", "true")}
          />
        ) : label === "Apple Music" ? (
          <MusicIcon aria-hidden className="me-[4px] h-[24px] w-[24px]" />
        ) : (
          <GlobeIcon aria-hidden className="me-[4px] h-[24px] w-[24px]" />
        )}

        {/* Non-breaking space keeps two-word labels ("Apple Music") from
            wrapping mid-name on narrow viewports. */}
        <span>{label.replace(" ", " ")}</span>
      </Link>
    </li>
  )
}

/** Loading is only visible past ~200ms so a fast response never flashes a
 * skeleton, and past ~6s the copy admits the request is taking a while
 * rather than leaving an unexplained skeleton on screen indefinitely. */
const useLoadingPhase = (isLoading: boolean) => {
  const [phase, setPhase] = useState<"skeleton" | "slow" | null>(null)

  useEffect(() => {
    if (!isLoading) return
    // Reset async (not synchronously in the effect body) so a second
    // loading run -- e.g. a manual retry -- doesn't carry over a stale
    // "slow" phase from the previous run before its own timers fire.
    const reset = setTimeout(() => setPhase(null), 0)
    const toSkeleton = setTimeout(() => setPhase("skeleton"), 200)
    const toSlow = setTimeout(() => setPhase("slow"), 6000)
    return () => {
      clearTimeout(reset)
      clearTimeout(toSkeleton)
      clearTimeout(toSlow)
    }
  }, [isLoading])

  return isLoading ? phase : null
}

const UpcomingEventsSkeleton = () => (
  <ul aria-hidden className="mt-1 flex flex-col gap-2 text-sm">
    {[0, 1].map((i) => (
      <li
        key={i}
        className="flex items-start gap-2 border-b border-slate-700/50 pb-2 last:border-b-0"
      >
        <span className="h-4 w-14 shrink-0 animate-pulse rounded bg-slate-700/50" />
        <div className="flex min-w-0 flex-1 flex-col gap-1.5 py-0.5">
          <span className="h-3.5 w-2/3 animate-pulse rounded bg-slate-700/50" />
          <span className="h-3 w-1/3 animate-pulse rounded bg-slate-700/40" />
        </div>
        <span className="ms-auto h-4 w-12 shrink-0 animate-pulse rounded bg-slate-700/50" />
      </li>
    ))}
  </ul>
)

const UpcomingEvents = (
  props: FutureEventsState & { onRetry?: () => void }
) => {
  const { status, onRetry } = props
  const events = status === "loaded" ? props.events : []
  const loadingPhase = useLoadingPhase(status === "loading")
  const shouldReduceMotion = useReducedMotion()
  // Simple opacity crossfade between statuses -- this is a small utility
  // list, not a hero moment, so no spatial motion, per the dashboard-tier
  // motion budget (micro-interactions only). mode="wait" avoids the old
  // and new status content overlapping mid-transition; initial={false}
  // skips animating whatever status happens to render first.
  const fade = {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: shouldReduceMotion ? 0 : 0.15 },
  }

  return (
    <div className="mt-4" aria-busy={status === "loading"}>
      <h3 className="text-xs tracking-wide text-slate-400 uppercase">
        Upcoming events
      </h3>

      <AnimatePresence mode="wait" initial={false}>
        {status === "loading" && (
          <motion.div key="loading" {...fade}>
            <span className="sr-only">Loading upcoming events…</span>
            {loadingPhase !== null && <UpcomingEventsSkeleton />}
            {loadingPhase === "slow" && (
              <p className="mt-1 text-sm text-slate-500">Still loading…</p>
            )}
          </motion.div>
        )}

        {status === "error" && (
          <motion.div
            key="error"
            {...fade}
            className="mt-1 flex items-center justify-between gap-2 text-sm"
          >
            <span className="text-slate-400">Couldn't load upcoming events.</span>
            {onRetry && (
              <button
                type="button"
                onClick={onRetry}
                className="shrink-0 text-(--text-link) underline underline-offset-2"
              >
                Try again
              </button>
            )}
          </motion.div>
        )}

        {status === "loaded" && (
          <motion.div key="loaded" {...fade}>
            {events.length ? (
              <ul className="mt-1 flex flex-col gap-2 text-sm">
                {events.map((e) => {
                  const venueLine =
                    [e.venue?.name, e.venue?.city].filter(Boolean).join(", ") ||
                    "—"
                  return (
                    <li
                      key={e.id}
                      className="flex items-start gap-2 border-b border-slate-700/50 pb-2 last:border-b-0"
                    >
                      <span dir="ltr" className="shrink-0">
                        {e.datesPretty}
                      </span>
                      <div className="flex min-w-0 flex-1 flex-col">
                        <span className="truncate font-semibold text-slate-600">
                          {e.name}
                        </span>
                        <span className="truncate text-slate-500">
                          {venueLine}
                        </span>
                      </div>
                      {e.url && (
                        <Link
                          href={e.url}
                          target="_blank"
                          className="ms-auto shrink-0 text-(--text-link) no-underline dark:text-(--text-link)"
                        >
                          {eventLinkLabel(e)}
                        </Link>
                      )}
                    </li>
                  )
                })}
              </ul>
            ) : (
              <p className="mt-1 text-sm text-slate-500">No upcoming events</p>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

export { EventDetails }
