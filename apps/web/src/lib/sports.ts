import type { EventResponse } from "../hooks/eventsStream"

/** The eight leagues sports enrichment covers -- mirrors
 * apps/api/src/sports/types.rs's `League` enum. Values are the `league`
 * query param `/api/sports/enrichment` expects (matched case-
 * insensitively backend-side against its own lowercase db values, so any
 * casing works as long as it lowercases to the right db string -- these
 * just mirror the backend's own naming for consistency). */
const PRO_LEAGUES = ["NBA", "NFL", "NHL", "MLB", "WNBA"] as const
type ProLeague = (typeof PRO_LEAGUES)[number]
type League = ProLeague | "NCAA_FOOTBALL" | "NCAA_MENS_BASKETBALL" | "NCAA_WOMENS_BASKETBALL"

/** The event's league, if its *explicitly marked* primary classification
 * is Sports under a recognized league -- mirrors
 * apps/api/src/sports/types.rs's `major_league` field-for-field,
 * including requiring an explicit `primary: true` rather than falling
 * back to the first classification when none is marked (per that
 * function's doc comment on why sports enrichment is conservative where
 * music's `is_music_classified` is permissive), and its two detection
 * strategies:
 * - Pro majors: `subGenre` alone identifies the league.
 * - College: `subGenre` is a flat `"College"` regardless of sport
 *   (confirmed live -- real Ticketmaster college football *and*
 *   basketball events both carry `subGenre=College`), so `genre`
 *   disambiguates the sport, and -- since Ticketmaster carries no
 *   structured field distinguishing men's from women's college
 *   basketball at all -- `isWomensEvent` falls back to the event's own
 *   *name*, the one place that distinction actually shows up.
 * `null` for anything else. */
const majorLeagueFor = (event: EventResponse): League | null => {
  const primary = (event.classifications ?? []).find((c) => c.primary)
  if (!primary?.segment?.name || !primary.subGenre?.name) return null
  if (primary.segment.name.toLowerCase() !== "sports") return null

  const subGenre = primary.subGenre.name.toUpperCase()
  if ((PRO_LEAGUES as readonly string[]).includes(subGenre)) {
    return subGenre as ProLeague
  }

  if (subGenre !== "COLLEGE") return null
  switch (primary.genre?.name) {
    case "Football":
      return "NCAA_FOOTBALL"
    case "Basketball":
      return isWomensEvent(event) ? "NCAA_WOMENS_BASKETBALL" : "NCAA_MENS_BASKETBALL"
    default:
      return null
  }
}

/** Whether `event` is a women's college basketball game, per its own
 * name -- e.g. "...Women's Basketball", "...Womens Basketball" (both
 * spellings seen live). Defaults to men's when absent, matching the
 * unmarked-default convention Ticketmaster's own listings already use. */
const isWomensEvent = (event: EventResponse): boolean =>
  event.name.toLowerCase().includes("women")

/** Whether `event` is a real matchup worth attempting sports enrichment
 * for -- mirrors apps/api/src/sports/types.rs's `matchups_from`: a
 * recognized league plus at least two *named* performers. Confirmed live
 * during this feature's design (both for the pro majors and, separately,
 * for college football) that performer count is what actually separates
 * a real game from a non-game Sports-segment listing (season tickets,
 * hospitality packages, stadium tours), which share the same
 * classification shape as a real matchup. */
const isMajorLeagueMatchup = (event: EventResponse): boolean =>
  majorLeagueFor(event) !== null &&
  (event.performers ?? []).filter((p) => p.name).length >= 2

export { majorLeagueFor, isMajorLeagueMatchup, type League }
