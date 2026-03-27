import type {
  Event,
  Classification,
  Attraction,
  AmArtistFull,
} from "./lib/types"
import { Button } from "@workspace/ui/components/ui/Button"
import { Link } from "@workspace/ui/components/ui/Link"
import { useEvents } from "./components/events-provider"
import { useEffect, useState } from "react"
import {
  ArrowLeftIcon,
  SquareUserRoundIcon,
  MapPinIcon,
  ClockIcon,
  MicVocalIcon,
  BotIcon,
  HandMetalIcon,
  GuitarIcon,
} from "lucide-react"
import WikiLogo from "@/assets/wikipedia-w-brands-solid-full.svg"
import { ReactSVG } from "react-svg"
import { SocialIcon } from "react-social-icons/component"
import "react-social-icons/instagram"

const EventDetails = ({ eventData }: { eventData: Event }) => {
  const [artistInfo, setArtistInfo] = useState<AmArtistFull[]>([])
  const [loading, setLoading] = useState<boolean>(true)
  const [error, setError] = useState<Error | null>(null)
  const eventsContext = useEvents()

  useEffect(() => {
    let cancelled = false

    async function fetchData() {
      try {
        setLoading(true)
        const artistInfoQuery = eventData.attractions
          .map((attraction) => `name=${encodeURI(attraction.name)}`)
          .join("&")
        const res = await fetch(`/api/apple/artist?${artistInfoQuery}`)
        if (!res.ok) throw new Error("Request failed")
        const json = await res.json()
        if (!cancelled) setArtistInfo(json)
      } catch (e) {
        if (!cancelled) setError(e as Error)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    fetchData()

    return () => {
      cancelled = true // avoid setting state after unmount
    }
  }, [])
  return (
    <div className="relative">
      <div className="relative">
        <div className="absolute top-0 left-0 z-1 h-full w-full bg-linear-to-t from-black to-transparent opacity-85" />
        <img className="w-full" src={eventData.images[0].url} />
        <Button
          className="absolute top-2 left-2 z-2"
          onClick={() => eventsContext.setSelectedEvent(undefined)}
        >
          <ArrowLeftIcon aria-hidden className="h-4 w-4" />
        </Button>
        <ul className="absolute top-2 right-2 z-2">
          {/* {eventUniqueGenres.map((genre) => (
            <GenreBadge genre={genre} />
          ))} */}
        </ul>

        <h3 className="absolute bottom-2 left-2 z-2 text-base/7 font-semibold text-white">
          {eventData.name}
        </h3>
      </div>
      <div className="p-2">
        <div className="flex justify-between">
          <div className="flex items-center gap-1">
            <ClockIcon aria-hidden className="h-4 w-4" />
            <span>{eventData.datesPretty}</span>
          </div>
          <div className="flex items-center gap-1">
            <MapPinIcon aria-hidden className="h-4 w-4" />
            <span>{eventData.venue.name}</span>
          </div>
        </div>
        <h4>Featuring</h4>
        {eventData.attractions?.map((attraction) => (
          <AttractionCard attraction={attraction} />
        ))}
        {artistInfo.map((artist) => (
          <ArtistArtworkCard artwork={artist.artwork} size={320} />
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

const ArtistCard = ({ artist }: { artist: AmArtistFull }) => {
  const { artwork } = artist
  return (
    <div>
      {artwork && (
        <img
          className="w-full"
          src={artwork.url
            .replace("{h}", artwork.height.toString())
            .replace("{w}", artwork.width.toString())}
        />
      )}
    </div>
  )
}

const AttractionCard = ({ attraction }: { attraction: Attraction }) => {
  return (
    <div>
      {attraction.images && (
        <img className="w-full" src={attraction.images[0].url} />
      )}
      {attraction.name}
      <ul className="flex">
        {attraction.externalLinks?.wiki && (
          <li className="h-10 w-10">
            <Link href={attraction.externalLinks.wiki[0].url} target="_blank">
              <ReactSVG className="h-10 w-10" src={WikiLogo} />
            </Link>
          </li>
        )}
        {attraction.externalLinks?.homepage && (
          <li className="h-10 w-10">
            <Link
              href={attraction.externalLinks.homepage[0].url}
              target="_blank"
            >
              <SquareUserRoundIcon aria-hidden className="h-10 w-10" />
            </Link>
          </li>
        )}
        {attraction.externalLinks?.instagram && (
          <li className="h-10 w-10">
            <Link
              href={attraction.externalLinks.instagram[0].url}
              target="_blank"
            >
              <SocialIcon
                aria-hidden
                className="h-10 w-10"
                network="instagram"
                as="i"
              />
            </Link>
          </li>
        )}
      </ul>
    </div>
  )
}

const GenreBadge = ({ genre }: { genre: string }) => {
  let icon
  switch (genre) {
    case "Dance/Electronic":
      icon = <BotIcon aria-hidden className="h-10 w-10" />
      break
    case "Country":
      icon = <GuitarIcon aria-hidden className="h-10 w-10" />
      break
    case "Rock":
      icon = <HandMetalIcon aria-hidden className="h-10 w-10" />
      break
    case "Pop":
    default:
      icon = <MicVocalIcon aria-hidden className="h-10 w-10" />
  }
  return (
    <li className="flex rounded-md bg-rose-400 text-white">
      {icon}
      <span>{genre}</span>
    </li>
  )
}

export { EventDetails }
