import { TagGroup, TagList, type Key } from "react-aria-components"
import { Tag as FilterChip } from "@workspace/ui/components/ui/TagGroup"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { Button } from "@workspace/ui/components/ui/Button"
import { Popover } from "@workspace/ui/components/ui/Popover"
import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { MOTION_DURATION, MOTION_EASE } from "@workspace/ui/lib/motion"
import {
  useEventsContext,
  matchedPerformerGenres,
} from "./providers/eventsProvider"
import { type Classification } from "./lib/types"
import { FilterIcon } from "lucide-react"
import { DialogTrigger, Heading } from "react-aria-components"

const toKeySet = (keys: "all" | Set<Key>) =>
  keys === "all" ? new Set<string>() : new Set([...keys].map(String))

const EventFilter = () => {
  const {
    eventsByDate,
    visibleEventsByDate,
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
  const totalEventCount = allEvents.length
  const visibleEventCount = Object.values(visibleEventsByDate).flat().length

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
        <Button
          aria-label={`Filters, showing ${visibleEventCount} of ${totalEventCount} events`}
          variant="secondary"
          className="flex h-9 shrink-0 items-center gap-1.5 rounded-full px-3 max-md:before:absolute max-md:before:-inset-1 max-md:before:content-['']"
        >
          <FilterIcon aria-hidden className="block h-4 w-4 shrink-0" />
          <span className="text-sm font-medium">Filters</span>
          {visibleEventCount < totalEventCount && (
            <span className="text-sm text-muted-foreground tabular-nums">
              · {visibleEventCount} of {totalEventCount}
            </span>
          )}
        </Button>
        <Popover showArrow>
          <Dialog className="max-h-[inherit] w-[350px] overflow-auto p-4">
            <div className="mb-2 flex items-center justify-between gap-2">
              <Heading slot="title" className="m-0 text-lg font-semibold">
                Filters
              </Heading>
              {totalSelected > 0 && (
                <Button
                  onPress={clearFilters}
                  variant="secondary"
                  className="h-auto shrink-0 px-2 py-1 text-xs"
                >
                  Clear
                </Button>
              )}
            </div>
            {totalSelected > 0 && (
              <p className="mb-3 text-sm text-muted-foreground">
                <span className="font-semibold text-foreground tabular-nums">
                  {visibleEventCount}
                </span>{" "}
                event{visibleEventCount === 1 ? "" : "s"} match
              </p>
            )}
            <div className="flex flex-col gap-4">
              <div className="flex flex-col gap-2">
                <Heading className="text-xs font-semibold text-muted-foreground uppercase">
                  Category
                </Heading>
                <TagGroup
                  selectionMode="multiple"
                  selectedKeys={activeClassifications}
                  onSelectionChange={(keys) =>
                    setActiveClassifications(toKeySet(keys))
                  }
                  escapeKeyBehavior="none"
                >
                  <TagList className="flex flex-wrap gap-1.5">
                    {Object.values(classifications).map((classification) => (
                      <FilterChip
                        key={classification.id}
                        id={classification.name}
                        textValue={`${classification.name} - ${classification.count}`}
                      >
                        {classification.name}
                        <span className="tabular-nums opacity-60">
                          {classification.count}
                        </span>
                      </FilterChip>
                    ))}
                  </TagList>
                </TagGroup>
              </div>

              {hasForYouOptions && (
                <div className="flex flex-col gap-2 border-t border-black/10 pt-4 dark:border-white/10">
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
                      <TagList className="flex flex-wrap gap-1.5">
                        {Object.entries(matchedArtists).map(([name, count]) => (
                          <FilterChip
                            key={name}
                            id={name}
                            textValue={`${name} - ${count}`}
                          >
                            {name}
                            <span className="tabular-nums opacity-60">
                              {count}
                            </span>
                          </FilterChip>
                        ))}
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
                      <TagList className="flex flex-wrap gap-1.5">
                        {Object.entries(matchedGenres).map(([genre, count]) => (
                          <FilterChip
                            key={genre}
                            id={genre}
                            textValue={`${genre} - ${count}`}
                          >
                            {genre}
                            <span className="tabular-nums opacity-60">
                              {count}
                            </span>
                          </FilterChip>
                        ))}
                      </TagList>
                    </TagGroup>
                  )}
                </div>
              )}
            </div>
          </Dialog>
        </Popover>
      </DialogTrigger>
    </>
  )
}

/** Renders on its own line below the title/trigger row (see
 * EventsDrawerHeader) rather than inline next to it -- packed into the
 * same flex row as the title, the chips' own width squeezed the title
 * into wrapping any time a filter was toggled. Animated in/out (height +
 * opacity, matching the "small UI appearing/disappearing" motion tier)
 * so that toggling the first/last filter shifts the date slider below it
 * smoothly instead of an instant jump -- the "jolt" this exists to avoid.
 * Still fully collapsed (zero height) at the zero-filter default; nothing
 * is permanently reserved. */
const EventFilterChips = () => {
  const {
    activeClassifications,
    setActiveClassifications,
    activeForYouArtists,
    setActiveForYouArtists,
    activeForYouGenres,
    setActiveForYouGenres,
  } = useEventsContext()
  const shouldReduceMotion = useReducedMotion()

  const totalSelected =
    activeClassifications.size +
    activeForYouArtists.size +
    activeForYouGenres.size

  const collapsed = { height: 0, opacity: 0 }
  const expanded = { height: "auto", opacity: 1 }
  const transition = {
    duration: shouldReduceMotion ? 0 : MOTION_DURATION.base,
    ease: MOTION_EASE.out,
  }

  return (
    <AnimatePresence initial={false}>
      {totalSelected > 0 && (
        <motion.div
          key="active-filter-chips"
          initial={collapsed}
          animate={expanded}
          exit={collapsed}
          transition={transition}
          className="overflow-hidden"
        >
          <div className="flex flex-wrap items-center gap-1.5 pb-2">
            {activeClassifications.size > 0 && (
              <motion.div layout="position" transition={transition}>
                <TagGroup
                  aria-label="Active classification filters"
                  onRemove={(keys) => {
                    const next = new Set(activeClassifications)
                    keys.forEach((key) => next.delete(String(key)))
                    setActiveClassifications(next)
                  }}
                >
                  <TagList className="flex flex-wrap gap-1.5">
                    {[...activeClassifications].map((name) => (
                      <FilterChip key={name} id={name}>
                        {name}
                      </FilterChip>
                    ))}
                  </TagList>
                </TagGroup>
              </motion.div>
            )}
            {activeForYouArtists.size > 0 && (
              <motion.div layout="position" transition={transition}>
                <TagGroup
                  aria-label="Active artist filters"
                  onRemove={(keys) => {
                    const next = new Set(activeForYouArtists)
                    keys.forEach((key) => next.delete(String(key)))
                    setActiveForYouArtists(next)
                  }}
                >
                  <TagList className="flex flex-wrap gap-1.5">
                    {[...activeForYouArtists].map((name) => (
                      <FilterChip key={name} id={name}>
                        {name}
                      </FilterChip>
                    ))}
                  </TagList>
                </TagGroup>
              </motion.div>
            )}
            {activeForYouGenres.size > 0 && (
              <motion.div layout="position" transition={transition}>
                <TagGroup
                  aria-label="Active genre filters"
                  onRemove={(keys) => {
                    const next = new Set(activeForYouGenres)
                    keys.forEach((key) => next.delete(String(key)))
                    setActiveForYouGenres(next)
                  }}
                >
                  <TagList className="flex flex-wrap gap-1.5">
                    {[...activeForYouGenres].map((genre) => (
                      <FilterChip key={genre} id={genre}>
                        {genre}
                      </FilterChip>
                    ))}
                  </TagList>
                </TagGroup>
              </motion.div>
            )}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

export { EventFilter, EventFilterChips }
