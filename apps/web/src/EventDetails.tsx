import type { Event, Classification, Attraction } from "./lib/types"
import { Button } from "@workspace/ui/components/ui/Button"
import { Link } from "@workspace/ui/components/ui/Link"
import { useEvents } from "./components/events-provider"
import {
  ArrowLeftIcon,
  SquareUserRoundIcon,
  MapPinIcon,
  ClockIcon,
  MicVocalIcon,
  BotIcon,
  GuitarIcon,
} from "lucide-react"
import WikiLogo from "@/assets/wikipedia-w-brands-solid-full.svg"
import { ReactSVG } from "react-svg"
import { SocialIcon } from "react-social-icons/component"
import "react-social-icons/instagram"

const EventDetails = ({ eventData }: { eventData: Event }) => {
  const eventsContext = useEvents()
  console.log(eventData.attractions)
  const eventUniqueGenres = eventData.attractions.reduce(
    (acc: string[], cur: Attraction) => {
      const genre = cur.classifications[0].genre.name
      if (!acc.includes(genre)) {
        acc.push(genre)
      }
      return acc
    },
    []
  )
  return (
    <div className="relative">
      <div className="relative">
        <div className="absolute top-0 left-0 z-1 h-full w-full bg-linear-to-t from-black to-transparent opacity-85" />
        <img
          className="w-full drop-shadow-red-400"
          src={eventData.images[0].url}
        />
        <Button
          className="absolute top-2 left-2 z-2"
          onClick={() => eventsContext.setSelectedEvent(undefined)}
        >
          <ArrowLeftIcon aria-hidden className="h-4 w-4" />
        </Button>
        <ul className="absolute top-2 right-2 z-2">
          {eventUniqueGenres.map((genre) => (
            <GenreBadge genre={genre} />
          ))}
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
        {eventData.attractions?.map((attraction) => (
          <AttractionCard attraction={attraction} />
        ))}
      </div>
    </div>
  )
}

const AttractionCard = ({ attraction }: { attraction: Attraction }) => {
  return (
    <div>
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
