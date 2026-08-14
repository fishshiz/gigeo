import { TagGroup, TagList, type Key } from "react-aria-components"
import { Tag as FilterChip } from "@workspace/ui/components/ui/TagGroup"
import { ToggleButton } from "@workspace/ui/components/ui/ToggleButton"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { Button } from "@workspace/ui/components/ui/Button"
import { Popover } from "@workspace/ui/components/ui/Popover"
import { AnimatePresence, motion, useReducedMotion } from "motion/react"
import { MOTION_DURATION, MOTION_EASE } from "@workspace/ui/lib/motion"
import { useEventsContext } from "./providers/eventsProvider"
import { type Classification } from "./lib/types"
import { FilterIcon, Sparkles } from "lucide-react"
import { DialogTrigger, Heading } from "react-aria-components"

const toKeySet = (keys: "all" | Set<Key>) =>
  keys === "all" ? new Set<string>() : new Set([...keys].map(String))

const EventFilter = () => {
  const {
    eventsByDate,
    visibleEventsByDate,
    activeClassifications,
    setActiveClassifications,
    forYouOnly,
    setForYouOnly,
  } = useEventsContext()

  const totalSelected = activeClassifications.size + (forYouOnly ? 1 : 0)

  const clearFilters = () => {
    setActiveClassifications(new Set())
    setForYouOnly(false)
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

  // A single "show me my matches" toggle, not a per-artist/genre browser
  // (see Phase 1 plan) -- scoped to currently-matched events only, zero
  // when nothing's matched (not connected, or no match in this search).
  const matchedEventCount = allEvents.filter((e) => e.matchedArtist).length

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

              {matchedEventCount > 0 && (
                <div className="flex flex-col gap-2 border-t border-black/10 pt-4 dark:border-white/10">
                  <Heading className="text-xs font-semibold text-muted-foreground uppercase">
                    For You
                  </Heading>
                  <ToggleButton
                    aria-label={`Only show events matched to your taste (${matchedEventCount})`}
                    isSelected={forYouOnly}
                    onChange={setForYouOnly}
                    className="flex w-fit shrink-0 items-center gap-1.5 rounded-full border border-neutral-200 bg-white px-3 py-1 text-xs font-medium text-neutral-700 transition hover:border-neutral-300 selected:border-transparent selected:bg-(--accent-bg) selected:text-(--text-on-accent) dark:border-neutral-600 dark:bg-neutral-900 dark:text-neutral-300 dark:hover:border-neutral-500"
                  >
                    <Sparkles aria-hidden className="h-3.5 w-3.5 shrink-0" />
                    For You
                    <span className="tabular-nums opacity-70">
                      {matchedEventCount}
                    </span>
                  </ToggleButton>
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
    forYouOnly,
    setForYouOnly,
  } = useEventsContext()
  const shouldReduceMotion = useReducedMotion()

  const totalSelected = activeClassifications.size + (forYouOnly ? 1 : 0)

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
            {forYouOnly && (
              <motion.div layout="position" transition={transition}>
                <TagGroup
                  aria-label="Active For You filter"
                  onRemove={() => setForYouOnly(false)}
                >
                  <TagList className="flex flex-wrap gap-1.5">
                    <FilterChip
                      id="for-you"
                      className="border-transparent bg-(--accent-bg) text-(--text-on-accent)"
                    >
                      For You
                    </FilterChip>
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
