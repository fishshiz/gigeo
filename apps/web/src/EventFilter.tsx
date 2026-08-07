import { TagGroup, Tag, TagList, type Key } from "react-aria-components"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { Button } from "@workspace/ui/components/ui/Button"
import { Tooltip } from "@workspace/ui/components/ui/Tooltip"
import { Popover } from "@workspace/ui/components/ui/Popover"
import {
  useEventsContext,
  matchedPerformerGenres,
} from "./providers/eventsProvider"
import { type Classification } from "./lib/types"
import { FilterIcon } from "lucide-react"
import { DialogTrigger, Heading, TooltipTrigger } from "react-aria-components"

const toKeySet = (keys: "all" | Set<Key>) =>
  keys === "all" ? new Set<string>() : new Set([...keys].map(String))

const EventFilter = () => {
  const {
    eventsByDate,
    activeClassifications,
    setActiveClassifications,
    activeForYouArtists,
    setActiveForYouArtists,
    activeForYouGenres,
    setActiveForYouGenres,
  } = useEventsContext()

  const totalSelected =
    activeClassifications.size +
    activeForYouArtists.size +
    activeForYouGenres.size

  const clearFilters = () => {
    setActiveClassifications(new Set())
    setActiveForYouArtists(new Set())
    setActiveForYouGenres(new Set())
  }

  const allEvents = Object.values(eventsByDate).flat()

  const classifications = allEvents
    .flatMap((e) => e.classifications || [])
    .reduce(
      (
        acc: Record<string, { name: string; id: string; count: number }>,
        curr: Classification
      ) => {
        const { name, id } = curr.segment ?? {}
        if (curr.primary && name && id) {
          if (!acc[name]) {
            acc[name] = { name, id, count: 1 }
          } else {
            acc[name].count += 1
          }
        }

        return acc
      },
      {}
    )

  // "For You" tag options are scoped to currently-matched events only --
  // this is a personalization filter, not a general performer/genre
  // browser (see Phase 1 plan). Empty when nothing's matched (not
  // connected, or no match in the current search results).
  const matchedEvents = allEvents.filter((e) => e.matchedArtist)

  const matchedArtists = matchedEvents.reduce(
    (acc: Record<string, number>, e) => {
      const name = e.matchedArtist!
      acc[name] = (acc[name] ?? 0) + 1
      return acc
    },
    {}
  )

  const matchedGenres = matchedEvents
    .flatMap(matchedPerformerGenres)
    .reduce((acc: Record<string, number>, genre) => {
      acc[genre] = (acc[genre] ?? 0) + 1
      return acc
    }, {})

  const hasForYouOptions =
    Object.keys(matchedArtists).length > 0 ||
    Object.keys(matchedGenres).length > 0

  return (
    <>
      <DialogTrigger>
        <TooltipTrigger>
          <Button
            aria-label="Filters"
            variant="secondary"
            className="relative !h-9 !w-9 shrink-0 max-md:before:absolute max-md:before:-inset-1 max-md:before:content-['']"
          >
            <FilterIcon aria-hidden className="block h-5 w-5 shrink-0" />
            {totalSelected > 0 && (
              <div className="absolute -top-2 -right-2 aspect-square h-4 rounded-full bg-(--accent-bg) text-xs text-(--text-on-accent)">
                {totalSelected}
              </div>
            )}
          </Button>
          <Tooltip>Filters</Tooltip>
        </TooltipTrigger>
        <Popover showArrow>
          <Dialog className="max-h-[inherit] w-[350px] overflow-auto p-4 outline outline-0">
            <Heading slot="title" className="m-0 mb-2 text-lg font-semibold">
              Filters
            </Heading>
            {totalSelected > 0 && (
              <Button
                onPress={clearFilters}
                variant="secondary"
                className="absolute top-4 right-4 h-auto px-2 py-1 text-xs"
              >
                Clear
              </Button>
            )}
            <div className="flex flex-col gap-4">
              <TagGroup
                selectionMode="multiple"
                selectedKeys={activeClassifications}
                onSelectionChange={(keys) =>
                  setActiveClassifications(toKeySet(keys))
                }
                escapeKeyBehavior="none"
              >
                <TagList>
                  {Object.values(classifications).map((classification) => (
                    <Tag
                      key={classification.id}
                      id={classification.name}
                      textValue={classification.name}
                    >{`${classification.name} - ${classification.count}`}</Tag>
                  ))}
                </TagList>
              </TagGroup>

              {hasForYouOptions && (
                <div className="flex flex-col gap-2">
                  <Heading className="text-xs font-semibold text-muted-foreground uppercase">
                    For You
                  </Heading>
                  {Object.keys(matchedArtists).length > 0 && (
                    <TagGroup
                      aria-label="Filter by matched artist"
                      selectionMode="multiple"
                      selectedKeys={activeForYouArtists}
                      onSelectionChange={(keys) =>
                        setActiveForYouArtists(toKeySet(keys))
                      }
                      escapeKeyBehavior="none"
                    >
                      <TagList>
                        {Object.entries(matchedArtists).map(
                          ([name, count]) => (
                            <Tag
                              key={name}
                              id={name}
                              textValue={name}
                            >{`${name} - ${count}`}</Tag>
                          )
                        )}
                      </TagList>
                    </TagGroup>
                  )}
                  {Object.keys(matchedGenres).length > 0 && (
                    <TagGroup
                      aria-label="Filter by matched genre"
                      selectionMode="multiple"
                      selectedKeys={activeForYouGenres}
                      onSelectionChange={(keys) =>
                        setActiveForYouGenres(toKeySet(keys))
                      }
                      escapeKeyBehavior="none"
                    >
                      <TagList>
                        {Object.entries(matchedGenres).map(
                          ([genre, count]) => (
                            <Tag
                              key={genre}
                              id={genre}
                              textValue={genre}
                            >{`${genre} - ${count}`}</Tag>
                          )
                        )}
                      </TagList>
                    </TagGroup>
                  )}
                </div>
              )}
            </div>
          </Dialog>
        </Popover>
      </DialogTrigger>
      {totalSelected > 0 && (
        <p className="text-xs text-muted-foreground">
          Filtering by:{" "}
          {[
            ...activeClassifications,
            ...activeForYouArtists,
            ...activeForYouGenres,
          ].join(", ")}
        </p>
      )}
    </>
  )
}

export { EventFilter }
