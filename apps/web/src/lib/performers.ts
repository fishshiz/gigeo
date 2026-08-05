import type { EventResponse } from "../hooks/eventsStream"

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

export { ticketmasterAttractionIds }
