import type { Event, Attraction, AmArtistFull } from "./lib/types"
import { Button } from "@workspace/ui/components/ui/Button"
import { Link } from "@workspace/ui/components/ui/Link"
import { useEffect, useState, useRef } from "react"
import { ResponsiveImage } from "@workspace/ui/components/ui/ResponsiveImage"
import {
  ArrowLeftIcon,
  GlobeIcon,
  MapPinIcon,
  DollarSignIcon,
  ClockIcon,
} from "lucide-react"
import WikiLogo from "@/assets/wikipedia-w-brands-solid-full.svg"
import IgLogo from "@/assets/instagram.svg"
import { ReactSVG } from "react-svg"
import "react-social-icons/instagram"
import type { EventResponse } from "./hooks/eventsStream"
import { useEventsContext } from "./providers/eventsProvider"
import { useEvents } from "./components/events-provider"

const EventDetails = ({ eventData }: { eventData: EventResponse }) => {
  const { attractions } = eventData
  const [artistInfo, setArtistInfo] = useState<AmArtistFull[]>([])
  const [futureEvents, setFutureEvents] = useState<Record<string, Event[]>>({})
  const [showStickyHeader, setShowStickyHeader] = useState(false)
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const { selectEvents } = useEventsContext()

  const eventsContext = useEvents()
  const { classifications } = eventData
  const segment =
    classifications && classifications.length > 0
      ? classifications[0].segment.name
      : null

  useEffect(() => {
    let cancelled = false

    async function fetchData() {
      try {
        const artistInfoQuery = eventData
          .attractions!.map(
            (attraction) => `name=${encodeURIComponent(attraction.name || "")}`
          )
          .join("&")
        console.log(artistInfoQuery)
        const res = await fetch(`/api/apple/artist?${artistInfoQuery}`)
        if (!res.ok) throw new Error("Request failed")
        const json = await res.json()
        if (!cancelled) setArtistInfo(json)
      } catch (e) {
        console.error("Failed to fetch artist info", e)
      }
    }

    async function fetchFutureEvents(id: string) {
      try {
        const res = await fetch(`/api/future-events?id=${id}`)
        if (!res.ok) throw new Error("Request failed")
        const json = await res.json()
        if (!cancelled) setFutureEvents((prev) => ({ ...prev, [id]: json }))
      } catch (e) {
        console.error("Failed to fetch future events", e)
      }
    }

    if (segment === "Music") {
      fetchData()
    }
    eventData.attractions?.forEach((attraction) => {
      if (attraction.id) {
        fetchFutureEvents(attraction.id)
      }
    })

    return () => {
      cancelled = true
    }
  }, [eventData, segment])

  // scroll handler for the pane
  useEffect(() => {
    const el = scrollRef.current
    if (!el) return

    const handleScroll = () => {
      // tweak threshold to taste (depends on hero height)
      const threshold = 160
      setShowStickyHeader(el.scrollTop > threshold)
    }

    handleScroll()
    el.addEventListener("scroll", handleScroll)
    return () => el.removeEventListener("scroll", handleScroll)
  }, [])

  return (
    <div ref={scrollRef} className="relative overflow-y-scroll">
      {/* sticky compact header inside the pane */}
      {showStickyHeader && (
        <div className="sticky top-0 z-20 flex items-center justify-between border-b border-slate-800/40 bg-slate-950/90 px-3 py-2 text-xs text-slate-100 backdrop-blur">
          <div className="flex min-w-0 items-center">
            <Button
              className="z-10 dark:border-(--color-border-subtle-dark-200) dark:bg-(--color-dusty-olive-dark-600)"
              variant="secondary"
              onClick={() => selectEvents([])}
            >
              <ArrowLeftIcon aria-hidden className="h-4 w-4" />
            </Button>
            <div className="ms-2 flex min-w-0 flex-col">
              <div className="truncate font-semibold">{eventData.name}</div>
              <div className="flex gap-2 text-[11px] text-slate-300">
                <span className="truncate">{eventData.datesPretty}</span>
                <span className="truncate">· {eventData.venue?.name}</span>
              </div>
            </div>
          </div>

          {eventData.url && (
            <Link
              href={eventData.url}
              target="_blank"
              variant="button"
              className="ml-2 shrink-0 rounded-full bg-(--color-toasted-almond-600) px-2 py-1 text-[11px] font-medium text-(--color-blush-rose-600) no-underline dark:bg-(--color-toasted-almond-dark-600) dark:text-(--color-text-primary-dark-600)"
            >
              Tickets
            </Link>
          )}
        </div>
      )}

      {/* back + top-right tickets over hero */}
      <Button
        className="absolute top-2 left-2 z-10 dark:border-(--color-border-subtle-dark-200) dark:bg-(--color-dusty-olive-dark-600)"
        variant="secondary"
        onClick={() => eventsContext.setSelectedEvent(undefined)}
      >
        <ArrowLeftIcon aria-hidden className="h-4 w-4" />
      </Button>

      {eventData.url && (
        <Link
          variant="button"
          className="absolute top-2 right-2 z-10 bg-(--color-toasted-almond-600) text-(--color-blush-rose-600) no-underline dark:bg-(--color-toasted-almond-dark-600) dark:text-(--color-text-primary-dark-600)"
          href={eventData.url}
          target="_blank"
        >
          Tickets
        </Link>
      )}

      <div className="relative">
        <div className="absolute top-0 left-0 z-[1] h-full w-full bg-gradient-to-t from-(--color-jet-black-900) to-transparent opacity-85 dark:from-(--color-bg-dark-900)" />
        <ResponsiveImage sources={eventData.images} alt="test" />
        <h3 className="absolute bottom-2 left-2 z-[2] text-2xl font-semibold text-(--color-ivory-100) dark:text-(--color-text-primary-dark-600)">
          {eventData.name}
        </h3>
      </div>

      {/* rest of your content exactly as before */}
      <div className="p-2">
        <div className="flex flex-col justify-between">
          <div className="flex items-center gap-1">
            <ClockIcon aria-hidden className="h-4 w-4" />
            <span>{eventData.datesPretty}</span>
          </div>
          <div className="flex items-center gap-1">
            <MapPinIcon aria-hidden className="h-4 w-4" />
            <span>{eventData.venue?.name}</span>
          </div>
          {eventData.priceRanges && (
            <div className="flex items-center gap-1">
              <DollarSignIcon aria-hidden className="h-4 w-4" />
              <span>
                {eventData.priceRanges[0].min} - {eventData.priceRanges[0].max}
              </span>
            </div>
          )}
        </div>
      </div>

      {artistInfo.length
        ? artistInfo.map((artist) => (
            <>
              <ArtistCard
                key={artist.id}
                artist={artist}
                similarArtists={artist.similar_artists}
                attraction={undefined}
                futureEvents={[]}
              />
            </>
          ))
        : attractions?.map((attraction) => {
            if (!attraction.id) return null
            const id = attraction.id
            return (
              <div key={id}>
                <h4 className="text-lg font-semibold">{attraction.name}</h4>
                <UpcomingEvents events={futureEvents[id] ?? []} />
              </div>
            )
          })}
    </div>
  )
}

type Artwork = {
  url: string
  bgColor?: string
}

type SimilarArtist = {
  id: string
  name: string
}

type ArtistCardProps = {
  artist: AmArtistFull
  similarArtists?: SimilarArtist[]
  artworkSize?: number
  attraction?: Attraction
  futureEvents?: Event[]
}

const buildArtworkUrl = (artwork: Artwork, size: number) => {
  return artwork.url.replace("{w}", String(size)).replace("{h}", String(size))
}

const normalizeBg = (bgColor?: string) => {
  if (!bgColor) return "#111827" // fallback
  return bgColor.startsWith("#") ? bgColor : `#${bgColor}`
}
export const ArtistCard: React.FC<ArtistCardProps> = ({
  artist,
  similarArtists = [],
  attraction = {},
  futureEvents = [],
  artworkSize = 200,
}) => {
  const { name, genres = [], artwork } = artist

  const imgUrl = buildArtworkUrl(artwork, artworkSize)
  const bgColor = normalizeBg(artwork.bgColor)
  const primaryGenre = genres[0]

  return (
    <div
      className="grid gap-4 p-4 text-slate-50 shadow-lg dark:text-(--color-text-secondary-600)"
      style={{ backgroundColor: bgColor }}
    >
      {/* Artwork block with bgColor */}
      <div className="relative flex shrink-0">
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
        <div>
          <h2 className="truncate text-xl font-semibold">{name}</h2>
          {primaryGenre && (
            <p className="mt-1 text-sm text-slate-300">{primaryGenre}</p>
          )}
          <ul className="flex flex-col">
            {attraction?.externalLinks?.wiki && (
              <ExternalLink
                url={attraction?.externalLinks?.wiki[0].url}
                label="Wikipedia"
              />
            )}
            {attraction?.externalLinks?.homepage && (
              <ExternalLink
                url={attraction?.externalLinks?.homepage[0].url}
                label="Website"
              />
            )}
            {attraction?.externalLinks?.instagram && (
              <ExternalLink
                url={attraction?.externalLinks?.instagram[0].url}
                label="Instagram"
              />
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
                  className="cursor-pointer rounded-full bg-slate-800/70 px-3 py-1 transition hover:bg-slate-700"
                >
                  {a.name}
                </li>
              ))}
            </ul>
          </div>
        )}

        <UpcomingEvents events={futureEvents} />
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
        className="my-1 flex items-center fill-(--color-toasted-almond-600) no-underline dark:fill-(--color-text-secondary-dark-600) dark:text-(--color-text-secondary-dark-600)"
      >
        {label === "Wikipedia" ? (
          <ReactSVG className="h-[24px] w-[24px]" src={WikiLogo} />
        ) : label === "Instagram" ? (
          <ReactSVG className="me-[4px] h-[24px] w-[24px]" src={IgLogo} />
        ) : (
          <GlobeIcon aria-hidden className="me-[4px] h-[24px] w-[24px]" />
        )}

        <span>{label}</span>
      </Link>
    </li>
  )
}

const UpcomingEvents = ({ events }: { events: Event[] }) => {
  console.log(events)
  return (
    <div className="mt-4">
      <h3 className="text-xs tracking-wide text-slate-400 uppercase">
        Upcoming events
      </h3>
      <ul className="mt-1 flex flex-col gap-2 text-sm">
        {events.map((e) => (
          <li
            key={e.id}
            className="flex items-start gap-2 border-b border-slate-700/50 pb-2 last:border-b-0"
          >
            <span>{e.datesPretty}</span>
            <div className="flex flex-col">
              <span className="font-semibold text-slate-600">{e.name}</span>
              <span className="text-slate-500">
                {e.venue?.name}, {e.venue?.city}
              </span>
            </div>
            <Link
              href={e.url}
              target="_blank"
              className="ml-auto text-(--color-toasted-almond-600) no-underline dark:text-(--color-text-secondary-dark-600)"
            >
              Tickets
            </Link>
          </li>
        ))}
      </ul>
    </div>
  )
}

export { EventDetails }
