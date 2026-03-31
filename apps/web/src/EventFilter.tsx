import { TagGroup, Tag, TagList } from "react-aria-components"
import { useEvents } from "./components/events-provider"
import { type Classification } from "./lib/types"
const EventFilter = () => {
  const eventsContext = useEvents()
  const { events } = eventsContext
  const classifications = Object.values(events)
    .flat()
    .flatMap((e) => e.classifications || [])
    .reduce((acc: string[], curr: Classification) => {
      if (
        curr.primary &&
        curr.segment.name &&
        !acc.includes(curr.segment.name)
      ) {
        acc.push(curr.segment.name)
      }
      return acc
    }, [])
  console.log(classifications)
  return (
    <TagGroup items={classifications}>
      <TagList className="gap 2 flex flex-wrap" items={classifications}>
        {classifications.map((classification) => (
          <Tag
            key={classification}
            id={classification}
            className="rounded-full bg-gray-200 px-2 py-1 text-sm text-gray-700"
          >
            {classification}
          </Tag>
        ))}
      </TagList>
    </TagGroup>
  )
}

export { EventFilter }
