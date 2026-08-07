import { TagGroup, Tag, TagList, type Key } from "react-aria-components"
import { Dialog } from "@workspace/ui/components/ui/Dialog"
import { Button } from "@workspace/ui/components/ui/Button"
import { Tooltip } from "@workspace/ui/components/ui/Tooltip"
import { Popover } from "@workspace/ui/components/ui/Popover"
import { useEventsContext } from "./providers/eventsProvider"
import { type Classification } from "./lib/types"
import { useState } from "react"
import { FilterIcon } from "lucide-react"
import { DialogTrigger, Heading, TooltipTrigger } from "react-aria-components"

const EventFilter = () => {
  const [selected, setSelected] = useState<Set<Key>>(new Set())

  const clearFilters = () => {
    setSelected(new Set())
  }
  const { eventsByDate } = useEventsContext()
  const classifications = Object.values(eventsByDate)
    .flat()
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
            {selected.size > 0 && (
              <div className="absolute -top-2 -right-2 aspect-square h-4 rounded-full bg-(--accent-bg) text-xs text-(--text-on-accent)">
                {selected.size}
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
            {selected.size > 0 && (
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
                selectedKeys={selected}
                onSelectionChange={(keys) =>
                  setSelected(keys === "all" ? new Set() : new Set(keys))
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
            </div>
          </Dialog>
        </Popover>
      </DialogTrigger>
      {selected.size > 0 && (
        <p className="text-xs text-muted-foreground">
          Filtering by: {[...selected].join(", ")}
        </p>
      )}
    </>
  )
}

export { EventFilter }
