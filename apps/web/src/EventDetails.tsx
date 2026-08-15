import type { AmArtistFull, ExternalLinks } from "./lib/types"
import { useDateFormatter } from "react-aria"
import { parseAbsolute } from "@internationalized/date"
import { Button } from "@workspace/ui/components/ui/Button"
import { Link } from "@workspace/ui/components/ui/Link"
import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { MOTION_DURATION, MOTION_EASE } from "@workspace/ui/lib/motion"
import { useCallback, useEffect, useId, useState, useRef } from "react"
import { ResponsiveImage } from "@workspace/ui/components/ui/ResponsiveImage"
import AppleMusicLogo from "./assets/Apple_Music_Icon_RGB_lg_073120.svg"
import { ForYouTag } from "./ForYouTag"

import {
  ArrowLeftIcon,
  GlobeIcon,
  MapPinIcon,
  DollarSignIcon,
  ClockIcon,
  TrophyIcon,
} from "lucide-react"
import WikiLogo from "@/assets/wikipedia-w-brands-solid-full.svg"
import IgLogo from "@/assets/instagram.svg"
import SpotifyLogo from "@/assets/Primary_Logo_Green_CMYK.svg"
import { ReactSVG } from "react-svg"
import "react-social-icons/instagram"
import type { EventResponse } from "./hooks/eventsStream"
import {
  amArtistFullSchema,
  teamEnrichmentSchema,
  type Performer,
  type TeamEnrichment,
} from "./hooks/eventsStreamSchema"
import { useEventsContext } from "./providers/eventsProvider"
import { useIsMobile } from "./providers/Breakpoint"
import { useCarouselIndex } from "./hooks/useCarouselIndex"
import { buildArtworkUrl, normalizeBg } from "./lib/artwork"
import { formatDate, formatTime } from "./lib/dates"
import { ticketmasterAttractionIds } from "./lib/performers"
import { majorLeagueFor, isMajorLeagueMatchup, type League } from "./lib/sports"

/** "Venue Name, City, ST" -- venue name plus a short city/state locator,
 * used anywhere a venue is referenced inline (the hero details line and
 * the upcoming-shows list both want the same shape). */
const venueLine = (venue: EventResponse["venue"]) =>
  [venue?.name, [venue?.city, venue?.stateCode].filter(Boolean).join(", ")]
    .filter(Boolean)
    .join(", ") || "—"

const formatPriceRange = (range: {
  currency: string
  min: number
  max: number
}) => {
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

/** Per-performer state for the on-demand `/api/artists/enrichment`
 * lookup -- fetched here rather than attached backend-side to every
 * streamed event, see `Performer.genres`'s doc comment
 * (apps/api/src/events/types.rs) for why. `{ status: "loaded", artist: null }`
 * is a genuine "no confident match" result, kept distinct from `"error"`
 * (the request itself failed) even though both render the same plain
 * fallback today -- see EventDetails' own render logic. */
type PerformerEnrichmentState =
  | { status: "loading" }
  | { status: "error" }
  | { status: "loaded"; artist: AmArtistFull | null }

/** Per-performer state for the on-demand `/api/sports/enrichment` lookup
 * -- same shape/rationale as `PerformerEnrichmentState` above, but for a
 * major-league matchup's teams instead of a music event's artists. Only
 * ever populated when `majorLeagueFor(eventData)` resolves a league (see
 * `lib/sports.ts`); an artist lookup never runs for those performers, so
 * the two enrichment kinds are mutually exclusive per event. */
type TeamEnrichmentState =
  | { status: "loading" }
  | { status: "error" }
  | { status: "loaded"; team: TeamEnrichment | null }

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
  // Computed once per eventData, not per performer -- both are event-level
  // (classification-derived) facts, not performer-level ones. See
  // `lib/sports.ts` for why this mirrors apps/api/src/sports/types.rs's
  // gating exactly rather than just checking `league !== null`.
  const league = majorLeagueFor(eventData)
  const isMatchup = isMajorLeagueMatchup(eventData)
  const [futureEvents, setFutureEvents] = useState<
    Record<string, FutureEventsState>
  >({})
  const [showStickyHeader, setShowStickyHeader] = useState(false)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const isMountedRef = useRef(true)
  const { selectEvents } = useEventsContext()
  const shouldReduceMotion = useReducedMotion()
  const isMobile = useIsMobile()

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
        setFutureEvents((prev) => ({
          ...prev,
          [id]: { status: "loaded", events },
        }))
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

  // Full canonical-artist enrichment (artwork/similar-artists/display
  // name/provider urls) for this event's performers, fetched on demand --
  // the stream only ever carries `performer.genres` eagerly, see
  // `Performer.genres`'s doc comment (apps/api/src/events/types.rs).
  // Keyed by performer name, same as the `/api/artists/enrichment` lookup
  // itself is.
  const [performerEnrichment, setPerformerEnrichment] = useState<
    Record<string, PerformerEnrichmentState>
  >({})

  const fetchPerformerEnrichment = useCallback(
    (name: string, ticketmasterAttractionId?: string | null) => {
      setPerformerEnrichment((prev) => ({
        ...prev,
        [name]: { status: "loading" },
      }))
      const params = new URLSearchParams({ name })
      if (ticketmasterAttractionId) {
        params.set("ticketmasterAttractionId", ticketmasterAttractionId)
      }
      fetch(`/api/artists/enrichment?${params}`)
        .then((res) => {
          if (!res.ok) throw new Error("Request failed")
          return res.json()
        })
        .then((json: unknown) => {
          if (!isMountedRef.current) return
          const result = amArtistFullSchema.nullable().safeParse(json)
          if (!result.success) {
            console.error(
              "Artist enrichment response didn't match the expected schema -- backend/frontend drift?",
              result.error,
              json
            )
            setPerformerEnrichment((prev) => ({
              ...prev,
              [name]: { status: "error" },
            }))
            return
          }
          setPerformerEnrichment((prev) => ({
            ...prev,
            [name]: { status: "loaded", artist: result.data },
          }))
        })
        .catch((e) => {
          console.error("Failed to fetch artist enrichment", e)
          if (!isMountedRef.current) return
          setPerformerEnrichment((prev) => ({
            ...prev,
            [name]: { status: "error" },
          }))
        })
    },
    []
  )

  useEffect(() => {
    // Skipped for a major-league matchup -- a team name is never going to
    // be a confident artist match (see `fetchTeamEnrichment` below for the
    // enrichment that actually applies here), so firing this too would
    // just be a wasted request per team on every sports event's detail
    // view.
    if (isMatchup) return
    ;(performers ?? [])
      .filter((performer): performer is Performer & { name: string } =>
        Boolean(performer.name)
      )
      .forEach((performer) =>
        fetchPerformerEnrichment(performer.name, performer.id)
      )
  }, [eventData, performers, isMatchup, fetchPerformerEnrichment])

  // Current record/standing for each team on a major-league matchup,
  // fetched on demand -- the sports-enrichment counterpart to
  // `performerEnrichment` above, see `apps/api/src/sports/mod.rs`'s
  // `get_team_enrichment`. Keyed by performer name, same as
  // `performerEnrichment` is.
  const [teamEnrichment, setTeamEnrichment] = useState<
    Record<string, TeamEnrichmentState>
  >({})

  const fetchTeamEnrichment = useCallback(
    (name: string, ticketmasterAttractionId: string, forLeague: League) => {
      setTeamEnrichment((prev) => ({
        ...prev,
        [name]: { status: "loading" },
      }))
      const params = new URLSearchParams({
        name,
        ticketmasterAttractionId,
        league: forLeague,
      })
      fetch(`/api/sports/enrichment?${params}`)
        .then((res) => {
          if (!res.ok) throw new Error("Request failed")
          return res.json()
        })
        .then((json: unknown) => {
          if (!isMountedRef.current) return
          const result = teamEnrichmentSchema.nullable().safeParse(json)
          if (!result.success) {
            console.error(
              "Sports enrichment response didn't match the expected schema -- backend/frontend drift?",
              result.error,
              json
            )
            setTeamEnrichment((prev) => ({
              ...prev,
              [name]: { status: "error" },
            }))
            return
          }
          setTeamEnrichment((prev) => ({
            ...prev,
            [name]: { status: "loaded", team: result.data },
          }))
        })
        .catch((e) => {
          console.error("Failed to fetch sports enrichment", e)
          if (!isMountedRef.current) return
          setTeamEnrichment((prev) => ({
            ...prev,
            [name]: { status: "error" },
          }))
        })
    },
    []
  )

  useEffect(() => {
    if (!isMatchup || !league) return
    // `ticketmasterAttractionId` is required by `/api/sports/enrichment`
    // (unlike the artist lookup, where it's optional) -- a named performer
    // with no id has nothing to key the match cache on backend-side, so
    // there's no point firing the request at all. See
    // apps/api/src/sports/mod.rs's `get_team_enrichment`.
    ;(performers ?? [])
      .filter(
        (performer): performer is Performer & { name: string; id: string } =>
          Boolean(performer.name && performer.id)
      )
      .forEach((performer) =>
        fetchTeamEnrichment(performer.name, performer.id, league)
      )
  }, [eventData, performers, isMatchup, league, fetchTeamEnrichment])

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
    <div ref={scrollRef} className="relative pb-4">
      {/* sticky compact header inside the pane */}
      <AnimatePresence>
        {showStickyHeader && (
          <motion.div
            key="sticky-header"
            initial={
              shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }
            }
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            transition={{
              duration: MOTION_DURATION.base,
              ease: MOTION_EASE.out,
            }}
            className="sticky top-0 z-sticky flex items-center justify-between gap-2 border-b border-slate-800/40 bg-slate-950/90 py-2 ps-3 pe-14 text-xs text-slate-100 backdrop-blur"
          >
            <div className="flex min-w-0 items-center">
              {/* bg-(--surface-secondary) is repeated under dark: because Button's
                  own "secondary" variant carries its own dark:bg-neutral-700 --
                  without the explicit dark: pair here, that wins the cascade over
                  the unprefixed override (confirmed empirically, not assumed). */}
              <Button
                aria-label="Back to events"
                className="z-10 touch-manipulation bg-(--surface-secondary) max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-[''] dark:border-(--color-border-subtle-dark-200) dark:bg-(--surface-secondary)"
                variant="secondary"
                onClick={() => selectEvents([])}
              >
                <ArrowLeftIcon
                  aria-hidden
                  className="h-4 w-4 rtl:-scale-x-100"
                />
              </Button>
              <div className="ms-2 flex min-w-0 flex-col">
                <div className="truncate font-semibold">{eventData.name}</div>
                <div className="flex gap-2 text-[11px] text-slate-300">
                  <span dir="ltr" className="truncate">
                    {eventData.datesPretty}
                  </span>
                  <span className="truncate">
                    · {eventData.venue?.name ?? "—"}
                  </span>
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
                Tickets
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
            initial={
              shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }
            }
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            transition={{
              duration: MOTION_DURATION.base,
              ease: MOTION_EASE.out,
            }}
          >
            <Button
              aria-label="Back to events"
              className="absolute start-2 top-2 z-10 touch-manipulation bg-(--surface-secondary) max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-[''] dark:border-(--color-border-subtle-dark-200) dark:bg-(--surface-secondary)"
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
            initial={
              shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }
            }
            animate={{ opacity: 1, y: 0 }}
            exit={shouldReduceMotion ? { opacity: 0 } : { opacity: 0, y: -8 }}
            transition={{
              duration: MOTION_DURATION.base,
              ease: MOTION_EASE.out,
            }}
          >
            <Link
              variant="button"
              className="absolute end-14 top-2 z-10 touch-manipulation bg-(--accent-bg) text-(--text-on-accent) no-underline max-md:before:absolute max-md:before:-inset-1.5 max-md:before:content-['']"
              href={eventData.url}
              target="_blank"
            >
              Tickets
            </Link>
          </motion.div>
        )}
      </AnimatePresence>

      <div className="relative">
        <div className="absolute start-0 top-0 z-[1] h-full w-full bg-gradient-to-t from-(--surface-scrim) to-transparent opacity-85" />
        <ResponsiveImage
          sources={eventData.images}
          alt={eventData.name}
          loading="eager"
        />
        <h3 className="absolute start-2 end-2 bottom-2 z-[2] line-clamp-2 text-2xl font-semibold text-(--text-on-scrim)">
          {eventData.name}
        </h3>
      </div>

      {/* rest of your content exactly as before */}
      <div className="p-2">
        {eventData.matchedArtist && (
          <div className="mb-2">
            <ForYouTag
              matchedArtist={eventData.matchedArtist}
              matchedVia={eventData.matchedVia}
            />
          </div>
        )}
        <div className="flex flex-col justify-between">
          <div className="flex items-center gap-1">
            <ClockIcon aria-hidden className="h-4 w-4 shrink-0" />
            <span dir="ltr" className="min-w-0 truncate">
              {eventData.datesPretty}
            </span>
          </div>
          <div className="flex items-center gap-1">
            <MapPinIcon aria-hidden className="h-4 w-4 shrink-0" />
            {eventData.venue?.url ? (
              <Link
                href={eventData.venue.url}
                target="_blank"
                className="min-w-0 truncate text-(--text-link)"
              >
                {venueLine(eventData.venue)}
              </Link>
            ) : (
              <span className="min-w-0 truncate">
                {venueLine(eventData.venue)}
              </span>
            )}
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
                    Tickets
                  </Link>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      {isMobile && performers && performers.length > 1 ? (
        <PerformerCarousel
          performers={performers}
          performerEnrichment={performerEnrichment}
          teamEnrichment={teamEnrichment}
          isMatchup={isMatchup}
          futureEventsFor={futureEventsFor}
          fetchFutureEvents={fetchFutureEvents}
        />
      ) : (
        performers?.map((performer, i) => (
          <PerformerDetails
            // Performer name isn't guaranteed unique on a bill (two "TBA"
            // openers, say) -- index breaks the tie without needing a
            // fabricated id.
            key={performer.id ?? `${performer.name}-${i}`}
            performer={performer}
            state={
              performer.name ? performerEnrichment[performer.name] : undefined
            }
            teamState={
              performer.name ? teamEnrichment[performer.name] : undefined
            }
            isMatchup={isMatchup}
            futureEvents={futureEventsFor(performer.id)}
            onRetryFutureEvents={
              performer.id
                ? () => fetchFutureEvents(performer.id as string)
                : undefined
            }
          />
        ))
      )}
    </div>
  )
}

/** Mobile-only alternative to the plain vertical stack: a single
 * full-width, horizontally snap-scrolling row (one performer per
 * "page") instead of stacking every performer's full ArtistCard
 * top-to-bottom -- a 4-5 act bill was a lot of scroll depth inside an
 * already height-constrained sheet. Desktop keeps the vertical stack
 * (more room, nothing to hide behind a swipe there). Only rendered when
 * there's more than one performer -- a single-performer bill has
 * nothing to carousel through. */
const PerformerCarousel = ({
  performers,
  performerEnrichment,
  teamEnrichment,
  isMatchup,
  futureEventsFor,
  fetchFutureEvents,
}: {
  performers: Performer[]
  performerEnrichment: Record<string, PerformerEnrichmentState>
  teamEnrichment: Record<string, TeamEnrichmentState>
  isMatchup: boolean
  futureEventsFor: (id: string | null | undefined) => FutureEventsState
  fetchFutureEvents: (id: string) => void
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const { activeIndex, scrollToIndex } = useCarouselIndex(containerRef)

  return (
    <div>
      {/* Dots sit above the track, not below it -- a below-the-fold "1 of
          4" under a full-height ArtistCard needed scrolling past the
          whole card just to notice the bill had more than one performer.
          Up top, it's the first thing visible and doubles as a "this
          swipes" cue, which plain text never was. Each dot is also a
          jump-to-slide control, not just an indicator. */}
      <div
        role="tablist"
        aria-label="Performers"
        className="flex items-center justify-center gap-2 pb-2"
      >
        {performers.map((performer, i) => (
          <button
            key={performer.id ?? `${performer.name}-${i}`}
            type="button"
            role="tab"
            aria-selected={i === activeIndex}
            aria-label={performer.name ?? `Performer ${i + 1}`}
            onClick={() => scrollToIndex(i)}
            className="relative touch-manipulation p-1.5 before:absolute before:-inset-1 before:content-['']"
          >
            <span
              aria-hidden
              className={
                i === activeIndex
                  ? "block h-1.5 w-4 rounded-full bg-(--accent-bg) transition-all"
                  : "block h-1.5 w-1.5 rounded-full bg-slate-600 transition-all"
              }
            />
          </button>
        ))}
      </div>
      <div
        ref={containerRef}
        role="region"
        aria-label="Performer details"
        tabIndex={0}
        className="flex snap-x snap-mandatory overflow-x-auto [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
      >
        {performers.map((performer, i) => (
          <div
            // See the vertical-stack fallback above for why index breaks
            // the tie on performer.id.
            key={performer.id ?? `${performer.name}-${i}`}
            className="w-full shrink-0 snap-center"
          >
            <PerformerDetails
              performer={performer}
              state={
                performer.name
                  ? performerEnrichment[performer.name]
                  : undefined
              }
              teamState={
                performer.name ? teamEnrichment[performer.name] : undefined
              }
              isMatchup={isMatchup}
              futureEvents={futureEventsFor(performer.id)}
              onRetryFutureEvents={
                performer.id
                  ? () => fetchFutureEvents(performer.id as string)
                  : undefined
              }
            />
          </div>
        ))}
      </div>
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
  const { name, genres = [], artwork, apple_music_url, spotify_url } = artist

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
      // Card surface is now a fixed token (bg-(--surface-scrim)) instead of
      // the artist's own artwork color -- see EventDetails' bgColor removal.
      // dark:border swaps in a ring instead of relying on a shadow, since
      // shadows are close to invisible on a near-black surface (no contrast
      // between a black-on-black shadow and the background it's cast on).
      className="grid gap-4 bg-(--surface-scrim) p-4 text-(--text-on-scrim) shadow-lg dark:border dark:border-white/10 dark:shadow-none"
    >
      {/* Artwork tile: bgColor is scoped to just this small block (fill
          behind any transparent artwork edges, plus a soft glow) rather
          than the whole card, so a light/washed-out artist palette never
          has to carry body-text contrast. shadow-[${bgColor}] (a previous
          attempt at this) never rendered, for two independent reasons:
          Tailwind's JIT scanner only generates arbitrary-value classes it
          can see as a literal string in source -- a runtime template
          literal like `shadow-[${bgColor}]` isn't one, so no CSS rule for
          it ever gets generated at build time. And even if it had, a bare
          color is invalid for the box-shadow property, which requires at
          least an x/y offset -- `box-shadow: #1a2b3c` alone is dropped by
          the browser as invalid. Set as an inline boxShadow instead: it's
          a genuinely per-artist runtime value, and inline style is the
          one place Tailwind's static scanning was never going to apply. */}
      <div className="relative flex min-w-0 shrink-0 gap-4">
        {imgUrl && (
          <div
            className="overflow-hidden rounded-2xl"
            style={{
              width: artworkSize,
              height: artworkSize,
              backgroundColor: bgColor,
              boxShadow: `0 6px 12px -4px ${bgColor}80`,
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
        <div className="min-w-0 flex-1">
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
            {spotify_url && <ExternalLink url={spotify_url} label="Spotify" />}
          </ul>
        </div>
      </div>

      {/* Text / metadata */}
      <div className="flex min-w-0 flex-1 flex-col justify-between">
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

/** A rough silhouette of `ArtistCard`'s own layout (artwork block + name +
 * genre lines) -- shown only past `useLoadingPhase`'s ~200ms delay, same
 * gate `UpcomingEventsSkeleton` uses, so a fast on-demand enrichment fetch
 * never flashes it. */
const ArtistCardSkeleton = () => (
  <div aria-hidden className="grid gap-4 p-4 dark:border dark:border-white/10">
    <div className="flex min-w-0 shrink-0 gap-4">
      <span className="h-[200px] w-[200px] shrink-0 animate-pulse rounded-2xl bg-slate-700/50" />
      <div className="min-w-0 flex-1 py-1">
        <span className="block h-5 w-2/3 animate-pulse rounded bg-slate-700/50" />
        <span className="mt-2 block h-3.5 w-1/3 animate-pulse rounded bg-slate-700/40" />
      </div>
    </div>
  </div>
)

type TeamCardProps = {
  team: TeamEnrichment
  futureEvents?: FutureEventsState
  onRetryFutureEvents?: () => void
}

/** Rows shown above/below the current team by default, before the
 * standings table needs an explicit "show more" -- a bare position
 * number wasn't useful on its own, but the *full* conference (up to ~15
 * teams for a pro league, more for some college conferences) is more
 * table than a card needs to open with. Mirrors `UpcomingEvents`'s own
 * truncate-then-expand pattern. */
const STANDINGS_VISIBLE_WINDOW = 2

/** A matchup team's slot in the details view -- the sports-enrichment
 * counterpart to `ArtistCard`, same surface/shell for visual consistency
 * but with a team's actual shape (no artwork, similar teams, or external
 * links -- its current record and where it sits in its conference/
 * division) instead of an artist's. */
const TeamCard: React.FC<TeamCardProps> = ({
  team,
  futureEvents = NO_FUTURE_EVENTS,
  onRetryFutureEvents,
}) => {
  const { teamName, record, groupName, standings } = team
  const [expanded, setExpanded] = useState(false)
  const standingsId = useId()

  const currentIndex = standings.findIndex((s) => s.teamName === teamName)
  // Collapsed size is fixed (independent of `expanded`) so toggling never
  // changes which count decides whether the button renders at all -- that
  // in turn is what keeps the button mounted across the toggle (see below).
  const collapsedCount =
    currentIndex === -1
      ? standings.length
      : Math.min(standings.length, STANDINGS_VISIBLE_WINDOW * 2 + 1)
  const canToggle = collapsedCount < standings.length
  const visibleStandings =
    expanded || currentIndex === -1
      ? standings
      : standings.filter(
          (_, i) => Math.abs(i - currentIndex) <= STANDINGS_VISIBLE_WINDOW
        )

  return (
    <div className="grid gap-4 bg-(--surface-scrim) p-4 text-(--text-on-scrim) shadow-lg dark:border dark:border-white/10 dark:shadow-none">
      <div className="flex min-w-0 items-center gap-4">
        {/* No team-crest data exists to put here (see `TeamEnrichment`) --
            sized and tinted to carry comparable visual weight to
            ArtistCard's artwork tile despite being iconography, not a
            photo, so the two enrichment kinds don't read as different
            products when swiped between in the mobile carousel. */}
        <span
          aria-hidden
          className="flex h-24 w-24 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-(--accent-bg)/25 to-slate-800 shadow-[0_6px_12px_-4px_rgba(0,0,0,0.4)]"
        >
          <TrophyIcon className="h-10 w-10 text-slate-200" />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-xl font-semibold">{teamName}</h2>
          {groupName && (
            <p className="mt-1 truncate text-sm text-slate-300">
              {groupName}
            </p>
          )}
          <div className="mt-2">
            <h3 className="text-xs tracking-wide text-slate-400 uppercase">
              Record
            </h3>
            <p dir="ltr" className="text-base font-semibold tabular-nums">
              {record}
            </p>
          </div>
        </div>
      </div>

      {visibleStandings.length > 0 && (
        <div>
          <h3 className="text-xs tracking-wide text-slate-400 uppercase">
            Standings
          </h3>
          <table id={standingsId} className="mt-1 w-full text-sm">
            <caption className="sr-only">{groupName ?? "Standings"}</caption>
            <thead>
              <tr className="text-xs text-slate-500">
                <th scope="col" className="w-8 py-1 text-left font-normal">
                  Pos
                </th>
                <th scope="col" className="py-1 text-left font-normal">
                  Team
                </th>
                <th scope="col" className="py-1 text-right font-normal">
                  Record
                </th>
              </tr>
            </thead>
            <tbody>
              {visibleStandings.map((entry) => (
                <tr
                  key={entry.teamName}
                  aria-current={entry.teamName === teamName ? "true" : undefined}
                  className={
                    entry.teamName === teamName
                      ? "bg-(--accent-bg)/15 font-semibold"
                      : "text-slate-300"
                  }
                >
                  <td
                    dir="ltr"
                    className="w-8 rounded-s px-1.5 py-1 tabular-nums text-slate-400"
                  >
                    {entry.standingPosition ?? "–"}
                  </td>
                  <td className="truncate px-1.5 py-1">{entry.teamName}</td>
                  <td
                    dir="ltr"
                    className="rounded-e px-1.5 py-1 text-right tabular-nums text-slate-400"
                  >
                    {entry.record}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {canToggle && (
            <button
              type="button"
              aria-expanded={expanded}
              aria-controls={standingsId}
              onClick={() => setExpanded((prev) => !prev)}
              className="relative mt-2 touch-manipulation text-sm text-(--text-link) underline underline-offset-2 before:absolute before:-inset-3 before:content-['']"
            >
              {expanded ? "Show fewer teams" : "Show full standings"}
            </button>
          )}
        </div>
      )}

      <UpcomingEvents {...futureEvents} onRetry={onRetryFutureEvents} />
    </div>
  )
}

/** A rough silhouette of `TeamCard`'s own layout, same threshold/purpose
 * as `ArtistCardSkeleton`. */
const TeamCardSkeleton = () => (
  <div aria-hidden className="grid gap-4 p-4 dark:border dark:border-white/10">
    <div className="flex min-w-0 items-center gap-4">
      <span className="h-24 w-24 shrink-0 animate-pulse rounded-2xl bg-slate-700/50" />
      <div className="min-w-0 flex-1 py-1">
        <span className="block h-5 w-2/3 animate-pulse rounded bg-slate-700/50" />
        <span className="mt-2 block h-3.5 w-1/3 animate-pulse rounded bg-slate-700/40" />
      </div>
    </div>
  </div>
)

/** One performer's slot in the details view: the rich `ArtistCard`/
 * `TeamCard` (depending on `isMatchup`) once its on-demand enrichment
 * resolves to a real match, a skeleton while that's in flight, or the
 * plain name-plus-upcoming-events fallback otherwise (no match found, the
 * fetch errored, or the performer has no name to look up at all). A real
 * component (not inlined in the parent's `.map`) because `useLoadingPhase`
 * is a hook -- can't call one conditionally per array item outside a
 * component boundary. */
const PerformerDetails = ({
  performer,
  state,
  teamState,
  isMatchup,
  futureEvents,
  onRetryFutureEvents,
}: {
  performer: Performer
  state: PerformerEnrichmentState | undefined
  teamState: TeamEnrichmentState | undefined
  isMatchup: boolean
  futureEvents: FutureEventsState
  onRetryFutureEvents?: () => void
}) => {
  const activeStatus = isMatchup ? teamState?.status : state?.status
  const loadingPhase = useLoadingPhase(activeStatus === "loading")

  let content: React.ReactNode = null

  if (isMatchup) {
    if (teamState?.status === "loaded" && teamState.team) {
      content = (
        <TeamCard
          team={teamState.team}
          futureEvents={futureEvents}
          onRetryFutureEvents={onRetryFutureEvents}
        />
      )
    }
  } else if (state?.status === "loaded" && state.artist) {
    content = (
      <ArtistCard
        artist={state.artist}
        similarArtists={state.artist.similar_artists}
        externalLinks={performer.externalLinks ?? undefined}
        futureEvents={futureEvents}
        onRetryFutureEvents={onRetryFutureEvents}
      />
    )
  }

  if (!content && activeStatus === "loading") {
    if (loadingPhase === null) return null
    content = isMatchup ? <TeamCardSkeleton /> : <ArtistCardSkeleton />
  }

  if (!content) {
    if (!performer.id) return null
    content = (
      <div>
        <h2 className="text-lg font-semibold">{performer.name}</h2>
        <UpcomingEvents {...futureEvents} onRetry={onRetryFutureEvents} />
      </div>
    )
  }

  // A single stable live region wrapping every branch (skeleton, loaded
  // card, or the plain fallback) -- rather than each branch returning its
  // own top-level element -- so the skeleton-to-card swap is a DOM mutation
  // *inside* an already-live region and gets announced, instead of being a
  // silent pop-in the way an unmounting/remounting top-level node would be.
  return (
    <div
      aria-live="polite"
      aria-busy={activeStatus === "loading"}
    >
      {activeStatus === "loading" && (
        <span className="sr-only">
          {isMatchup ? "Loading team details…" : "Loading artist details…"}
        </span>
      )}
      {content}
    </div>
  )
}

const ExternalLink = ({ url, label }: { url: string; label: string }) => {
  return (
    <li className="flex">
      <Link
        href={url}
        target="_blank"
        className="my-1 flex items-center fill-(--text-on-scrim) text-(--text-on-scrim) no-underline dark:fill-(--text-on-scrim) dark:text-(--text-on-scrim)"
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
          <ReactSVG
            className="me-[4px] h-[24px] w-[24px]"
            beforeInjection={(svg) => {
              svg.setAttribute("width", String("24"))
              svg.setAttribute("height", String("24"))
              svg.setAttribute(
                "viewBox",
                svg.getAttribute("viewBox") || "0 0 24 24"
              )
              svg.setAttribute("aria-hidden", "true")
            }}
            src={AppleMusicLogo}
          />
        ) : label === "Spotify" ? (
          <ReactSVG
            className="me-[4px] h-[24px] w-[24px]"
            src={SpotifyLogo}
            beforeInjection={(svg) => svg.setAttribute("aria-hidden", "true")}
          />
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

/** Keeps a single artist's upcoming-events list from dwarfing the rest of
 * the performer card -- most artists on tour have far more than this many
 * future dates, and printing all of them push everything else below the
 * fold. */
const UPCOMING_EVENTS_VISIBLE_COUNT = 5

const UpcomingEvents = (
  props: FutureEventsState & { onRetry?: () => void }
) => {
  const dateFormatter = useDateFormatter({
    month: "short",
    day: "numeric",
  })
  const { status, onRetry } = props
  const events = status === "loaded" ? props.events : []
  const loadingPhase = useLoadingPhase(status === "loading")
  const shouldReduceMotion = useReducedMotion()
  const [expanded, setExpanded] = useState(false)
  const listId = useId()
  const visibleEvents =
    expanded || events.length <= UPCOMING_EVENTS_VISIBLE_COUNT
      ? events
      : events.slice(0, UPCOMING_EVENTS_VISIBLE_COUNT)
  // Simple opacity crossfade between statuses -- this is a small utility
  // list, not a hero moment, so no spatial motion, per the dashboard-tier
  // motion budget (micro-interactions only). mode="wait" avoids the old
  // and new status content overlapping mid-transition; initial={false}
  // skips animating whatever status happens to render first.
  const fade = {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
    transition: { duration: shouldReduceMotion ? 0 : MOTION_DURATION.fast },
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
            <span className="text-slate-400">
              Couldn't load upcoming events.
            </span>
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
              <ul id={listId} className="mt-1 flex flex-col gap-2 text-sm">
                {visibleEvents.map((e) => {
                  return (
                    <li
                      key={e.id}
                      className="grid grid-cols-6 gap-4 border-b border-slate-700/50 pb-2 last:border-b-0"
                    >
                      <span dir="ltr" className="shrink-0">
                        {e.dates
                          ? e.dates.includes("T")
                            ? dateFormatter.format(
                                parseAbsolute(e.dates, "UTC").toDate()
                              )
                            : formatDate(e.dates)
                          : e.datesPretty}
                      </span>
                      <div className="col-start-2 col-end-6 flex min-w-0 flex-1 flex-col">
                        <span className="truncate font-semibold text-slate-600">
                          {e.name}
                        </span>
                        <span className="truncate text-slate-500">
                          {venueLine(e.venue)}
                        </span>
                      </div>
                      {e.url && (
                        <Link
                          href={e.url}
                          target="_blank"
                          className="ms-auto shrink-0 text-(--text-link) no-underline dark:text-(--text-link)"
                        >
                          Tickets
                        </Link>
                      )}
                    </li>
                  )
                })}
              </ul>
            ) : (
              <p className="mt-1 text-sm text-slate-500">No upcoming events</p>
            )}
            {events.length > UPCOMING_EVENTS_VISIBLE_COUNT && (
              <button
                type="button"
                aria-expanded={expanded}
                aria-controls={listId}
                onClick={() => setExpanded((prev) => !prev)}
                className="relative mt-2 touch-manipulation text-sm text-(--text-link) underline underline-offset-2 before:absolute before:-inset-3 before:content-['']"
              >
                {expanded
                  ? "Show fewer"
                  : `Show ${events.length - UPCOMING_EVENTS_VISIBLE_COUNT} more`}
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

export { EventDetails }
