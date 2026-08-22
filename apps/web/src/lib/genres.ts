import { type EventsByDate } from "@/hooks/eventsStream"

/** Every genre currently attached to a nearby performer, deduped and
 * sorted -- sourced from events already streamed into `eventsByDate`
 * (`performer.genres`, wired for #102) rather than a separate fetch, so the
 * playlist genre picker only ever offers genres real nearby artists
 * actually have. */
export function nearbyGenres(eventsByDate: EventsByDate): string[] {
  const genres = new Set<string>()
  for (const events of Object.values(eventsByDate)) {
    for (const event of events) {
      for (const performer of event.performers ?? []) {
        for (const genre of performer.genres ?? []) {
          genres.add(genre)
        }
      }
    }
  }
  return [...genres].sort()
}
