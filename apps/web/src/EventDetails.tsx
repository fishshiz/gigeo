import type { Event, Attraction, AmArtistFull } from "./lib/types"
import { Button } from "@workspace/ui/components/ui/Button"
import { Link } from "@workspace/ui/components/ui/Link"
import { useEvents } from "./components/events-provider"
import { useEffect, useState } from "react"
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

const EventDetails = ({ eventData }: { eventData: Event }) => {
  const { attractions } = eventData
  const [artistInfo, setArtistInfo] = useState<AmArtistFull[]>([])

  const eventsContext = useEvents()

  useEffect(() => {
    let cancelled = false

    async function fetchData() {
      try {
        const artistInfoQuery = eventData.attractions
          .map((attraction) => `name=${encodeURIComponent(attraction.name)}`)
          .join("&")
        const res = await fetch(`/api/apple/artist?${artistInfoQuery}`)
        if (!res.ok) throw new Error("Request failed")
        const json = await res.json()
        if (!cancelled) setArtistInfo(json)
      } catch (e) {}
    }

    fetchData()

    return () => {
      cancelled = true // avoid setting state after unmount
    }
  }, [])
  console.log(eventData)
  return (
    <div className="overflow-y-scroll">
      <div className="relative">
        <div className="absolute top-0 left-0 z-1 h-full w-full bg-linear-to-t from-(--color-jet-black-900) to-transparent opacity-85 dark:from-(--color-bg-dark-900)" />
        <ResponsiveImage sources={eventData.images} alt="test" />
        <Button
          className="absolute top-2 left-2 z-2 dark:border-(--color-border-subtle-dark-200) dark:bg-(--color-dusty-olive-dark-600)"
          variant="secondary"
          onClick={() => eventsContext.setSelectedEvent(undefined)}
        >
          <ArrowLeftIcon aria-hidden className="h-4 w-4" />
        </Button>
        {eventData.url && (
          <Link
            variant="button"
            className="absolute top-2 right-2 z-2 bg-(--color-toasted-almond-600) text-(--color-blush-rose-600) no-underline dark:bg-(--color-toasted-almond-dark-600) dark:text-(--color-text-primary-dark-600)"
            href={eventData.url}
            target="_blank"
          >
            Tickets
          </Link>
        )}
        <ul className="absolute top-2 right-2 z-2">
          {/* {eventUniqueGenres.map((genre) => (
            <GenreBadge genre={genre} />
          ))} */}
        </ul>

        <h3 className="absolute bottom-2 left-2 z-2 text-2xl font-semibold text-(--color-jet-black-600) dark:text-(--color-text-primary-dark-600)">
          {eventData.name}
        </h3>
      </div>
      <div className="p-2">
        <div className="flex flex-col justify-between">
          <div className="flex items-center gap-1">
            <ClockIcon aria-hidden className="h-4 w-4" />
            <span>{eventData.datesPretty}</span>
          </div>
          <div className="flex items-center gap-1">
            <MapPinIcon aria-hidden className="h-4 w-4" />
            <span>{eventData.venue.name}</span>
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

        {artistInfo.map((artist, idx) => (
          <ArtistCard
            key={artist.id}
            artist={artist}
            similarArtists={artist.similar_artists}
            attraction={attractions[idx]}
          />
        ))}
      </div>
    </div>
  )
}

export const ArtistArtworkCard = ({
  artwork,
  size = 240,
}: {
  artwork: AmArtistFull["artwork"]
  size: number
}) => {
  const { url, bgColor } = artwork

  // Apple usually returns bgColor as "RRGGBB" (no '#'), so normalize it.
  const normalizedBg = bgColor
    ? bgColor.startsWith("#")
      ? bgColor
      : `#${bgColor}`
    : "#111827" // fallback (Tailwind slate-900-ish)

  // Replace {w} and {h} tokens in the URL
  const imgUrl = url.replace("{w}", String(size)).replace("{h}", String(size))

  return (
    <div
      className="relative flex items-center justify-center overflow-hidden rounded-xl"
      style={{
        width: size,
        height: size,
        backgroundColor: normalizedBg,
      }}
    >
      <img
        src={imgUrl}
        alt="Artist artwork"
        className="h-full w-full object-cover"
        loading="lazy"
      />
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
  attraction,
  artworkSize = 200,
}) => {
  const { name, genres = [], artwork } = artist

  const imgUrl = buildArtworkUrl(artwork, artworkSize)
  const bgColor = normalizeBg(artwork.bgColor)
  const primaryGenre = genres[0]
  const componentStyle = {
    "--tw-gradient-from": `${normalizeBg(artwork.bgColor)}80`,
  } as React.CSSProperties

  return (
    <div
      className="flex gap-4 rounded-2xl bg-linear-to-t to-transparent p-4 text-slate-50 shadow-lg dark:text-(--color-text-secondary-600)"
      style={componentStyle}
    >
      {/* Artwork block with bgColor */}
      <div className="relative flex shrink-0 flex-col">
        <div
          className="overflow-hidden rounded-2xl"
          style={{
            width: artworkSize,
            height: artworkSize,
            backgroundColor: bgColor,
          }}
        >
          <img
            src={imgUrl}
            alt={`${name} artwork`}
            className="h-full w-full object-cover"
            loading="lazy"
          />
        </div>
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

      {/* Text / metadata */}
      <div className="flex flex-1 flex-col justify-between">
        <div>
          <h2 className="truncate text-xl font-semibold">{name}</h2>
          {primaryGenre && (
            <p className="mt-1 text-sm text-slate-300">{primaryGenre}</p>
          )}
        </div>

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

export { EventDetails }
