import { TagGroup, Tag, TagList } from "react-aria-components"
import { useEvents } from "./components/events-provider"
import { type Classification } from "./lib/types"
const EventFilter = () => {
  const eventsContext = useEvents()
  const { events } = eventsContext
  const classifications = Object.values(events)
    .flat()
    .flatMap((e) => e.classifications || [])
    .reduce((acc: { id: string; name: string }[], curr: Classification) => {
      if (
        curr.primary &&
        curr.segment.name &&
        !acc.find((item) => item.name === curr.segment.name)
      ) {
        acc.push({ id: curr.segment.name, name: curr.segment.name })
      }
      return acc
    }, [])
  console.log(classifications)
  return (
    <TagGroup>
      <TagList className="gap 2 flex flex-wrap" items={classifications}>
        {classifications.map((classification) => (
          <Tag
            key={classification.id}
            id={classification.id}
            className="rounded-full bg-gray-200 px-2 py-1 text-sm text-gray-700"
          >
            {classification.name}
          </Tag>
        ))}
      </TagList>
    </TagGroup>
  )
}

export { EventFilter }
