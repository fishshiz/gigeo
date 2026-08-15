import type { EventResponse } from "../hooks/eventsStream"

/** The four major US leagues sports enrichment covers -- mirrors
 * apps/api/src/sports/types.rs's `League` enum. The string values are the
 * `league` query param `/api/sports/enrichment` expects (matched
 * case-insensitively backend-side against its own lowercase db values). */
const MAJOR_LEAGUES = ["NBA", "NFL", "NHL", "MLB"] as const
type League = (typeof MAJOR_LEAGUES)[number]

/** The event's major league, if its *explicitly marked* primary
 * classification is Sports under an allow-listed subGenre -- mirrors
 * apps/api/src/sports/types.rs's `major_league` field-for-field
 * (including requiring an explicit `primary: true`, not falling back to
 * the first classification when none is marked, per that function's doc
 * comment on why sports enrichment is conservative where music's
 * `is_music_classified` is permissive). `null` for anything else. */
const majorLeagueFor = (event: EventResponse): League | null => {
  const primary = (event.classifications ?? []).find((c) => c.primary)
  if (!primary?.segment?.name || !primary.subGenre?.name) return null
  if (primary.segment.name.toLowerCase() !== "sports") return null

  const subGenre = primary.subGenre.name.toUpperCase()
  return (MAJOR_LEAGUES as readonly string[]).includes(subGenre)
    ? (subGenre as League)
    : null
}

/** Whether `event` is a real major-league matchup worth attempting sports
 * enrichment for -- mirrors apps/api/src/sports/types.rs's
 * `matchups_from`: a major league plus at least two *named* performers.
 * Confirmed live during that feature's design that performer count is
 * what actually separates a real game from a non-game Sports-segment
 * listing (season tickets, hospitality packages, stadium tours), which
 * share the same classification shape as a real matchup. */
const isMajorLeagueMatchup = (event: EventResponse): boolean =>
  majorLeagueFor(event) !== null &&
  (event.performers ?? []).filter((p) => p.name).length >= 2

export { majorLeagueFor, isMajorLeagueMatchup, type League }
