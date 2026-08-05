import type { EventResponse } from "../hooks/eventsStream"
import type { ExternalLinks } from "./types"

/** Performer ids from `event` that are valid Ticketmaster attraction ids,
 * safe to pass to `/future-events`. A performer's `id` is only a
 * Ticketmaster attraction id when the event itself came from Ticketmaster
 * -- for a PredictHQ-sourced event it's a PredictHQ entity id instead, a
 * different namespace that `/future-events` (a Ticketmaster-only endpoint)
 * doesn't recognize (see apps/api/src/predicthq/normalize.rs). */
const ticketmasterAttractionIds = (event: EventResponse): string[] => {
  if (event.source !== "ticketmaster") return []
  return (event.performers ?? [])
    .map((performer) => performer.id)
    .filter((id): id is string => Boolean(id))
}

const normalizeForMatch = (name: string) => name.trim().toLowerCase()

/** Finds the Ticketmaster-provided external links (wiki/homepage/Instagram)
 * for the performer whose name matches `artistName`, case/whitespace
 * -insensitively. Used to attach those links to an Apple Music search
 * result, since the two are only ever matched by name -- Apple Music has
 * no concept of a Ticketmaster attraction id. `undefined` when there's no
 * matching performer or it has no external links (a PredictHQ-sourced
 * event's performers never do -- see
 * apps/api/src/predicthq/normalize.rs). */
const externalLinksForArtist = (
  performers: EventResponse["performers"],
  artistName: string
): ExternalLinks | undefined => {
  const target = normalizeForMatch(artistName)
  const match = performers?.find(
    (performer) =>
      performer.name && normalizeForMatch(performer.name) === target
  )
  return match?.externalLinks ?? undefined
}

export { ticketmasterAttractionIds, externalLinksForArtist }
