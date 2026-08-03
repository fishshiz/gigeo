import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
  type CalendarDate,
} from "@internationalized/date"
import type { EventResponse } from "../hooks/eventsStream"

// ---------------------------------------------------------------------------
// LocalDay — the calendar day an event or search boundary falls on in the
// viewer's own timezone (not necessarily the venue's — the app assumes the
// two approximate each other; see apps/web/CONTEXT.md). This is the single
// owner of that concept: every place that needs to bucket, query, or display
// something by local day goes through here.
// ---------------------------------------------------------------------------

const BARE_DATE = /^\d{4}-\d{2}-\d{2}$/
const HAS_OFFSET = /[Zz]|[+-]\d{2}:?\d{2}$/

/** Extracts the `YYYY-MM-DD` LocalDay from an arbitrary date-time string,
 * honoring `timeZone` (defaults to the viewer's own).
 *
 * `EventResponse.dates` comes through in three shapes, and only one of them
 * involves a real timezone conversion:
 *
 * - A bare `YYYY-MM-DD` (no time-of-day) — already the day, as written.
 * - A timezone-less local wall-clock string, e.g. `"2026-08-01T23:30:00"`
 *   (from `normalize_event`'s localDate+localTime fallback) — also already
 *   the day, as written. Routing this through `Date` would silently assume
 *   it's local to *this runtime*, then reformatting in an arbitrary target
 *   `timeZone` would double-convert it — reading the date substring
 *   directly avoids that entirely, and is correct regardless of what
 *   `timeZone` is passed or what zone the runtime itself is in.
 * - A genuine UTC/offset instant, e.g. `"2026-08-02T03:00:00Z"` — the only
 *   shape that actually needs conversion. An 11pm Eastern show is 3am UTC
 *   the *next* day; reading the UTC calendar day instead of the local one
 *   is exactly the bug this module exists to prevent (see reducers/events.ts
 *   history, and dateRangeToApiParams below for the matching search-side
 *   bug it exposed).
 */
const toLocalDay = (
  dateString: string,
  timeZone: string = getLocalTimeZone()
): string | null => {
  if (!HAS_OFFSET.test(dateString)) {
    const day = dateString.slice(0, 10)
    return BARE_DATE.test(day) ? day : null
  }

  const date = new Date(dateString)
  if (Number.isNaN(date.getTime())) return null

  // en-CA formats numeric dates as YYYY-MM-DD -- the standard idiom for
  // getting an ISO-ordered date out of Intl without hand-assembling one
  // from formatToParts.
  return new Intl.DateTimeFormat("en-CA", {
    timeZone,
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(date)
}

/** The LocalDay an event belongs to, or `"unknown"` if it has no (or an
 * unparseable) `dates` value. Used to bucket events for the drawer and to
 * key same-day/same-venue showtime grouping (`lib/groupEvents.ts`). */
const eventDateKey = (
  event: EventResponse,
  timeZone: string = getLocalTimeZone()
): string => {
  if (!event.dates) return "unknown"
  return toLocalDay(event.dates, timeZone) ?? "unknown"
}

const formatDate = (dateString: string) => {
  // Parse YYYY-MM-DD as UTC to avoid timezone shifts
  const [year, month, day] = dateString.split("-").map(Number)
  const date = new Date(Date.UTC(year, month - 1, day))

  const formatter = new DateFormatter("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC",
  })

  return formatter.format(date)
}

/** Ticketmaster events without a specific start time come through as a bare
 * `YYYY-MM-DD` (no "T"), which `parseAbsolute` rejects as invalid — those
 * have no time-of-day to format, only a date. */
const hasTimeComponent = (dateString: string) => dateString.includes("T")

const formatDateTime = (
  date: string,
  timeZone: string = getLocalTimeZone()
) => {
  if (!hasTimeComponent(date)) return formatDate(date)

  const parsedDate = parseAbsolute(date, "UTC")

  const dateFormatter = new DateFormatter("en-US", {
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone,
  })
  return dateFormatter.format(parsedDate.toDate())
}

const formatTime = (
  dateString: string,
  timeZone: string = getLocalTimeZone()
) => {
  if (!hasTimeComponent(dateString)) return null

  const parsedDate = parseAbsolute(dateString, "UTC")

  const formatter = new DateFormatter("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone,
  })
  return formatter.format(parsedDate.toDate())
}

/** Converts a user-picked local calendar-date range into the UTC instants
 * the `/concerts/stream` API expects, anchored to *local* midnight rather
 * than UTC midnight.
 *
 * `CalendarDate.toString()` + `"T00:00:00Z"` (the previous approach) treats
 * the selected date as if it were already UTC midnight -- in any timezone
 * behind UTC, that's several hours before the user's actual local midnight.
 * E.g. for a search starting "August 3" in US Eastern (UTC-4), the query
 * window silently began at 8pm on August 2 local time, pulling in any show
 * starting between 8pm and midnight on the day *before* the one the user
 * selected. This was invisible before the eventDateKey fix, which grouped
 * everything by UTC day too (masking the mismatch); now that grouping is
 * local, that spillover shows up as events under an unexpected date
 * heading a day earlier than the search start.
 *
 * `end` uses local midnight of the day *after* the selected end date
 * (rather than a same-day 23:59:59 local instant) as a simpler and
 * equally-correct upper bound -- the backend's date range end is treated
 * as inclusive, so this just adds a technically-unreachable extra instant
 * rather than risking a sub-second gap from constructing 23:59:59 by hand.
 */
const dateRangeToApiParams = (
  start: CalendarDate,
  end: CalendarDate,
  timeZone: string = getLocalTimeZone()
): { start: string; end: string } => ({
  start: start.toDate(timeZone).toISOString(),
  end: end.add({ days: 1 }).toDate(timeZone).toISOString(),
})

const groupDatesByWeek = (dates: string[]): string[][] => {
  const groups: string[][] = []

  for (let i = 0; i < dates.length; i += 7) {
    groups.push(dates.slice(i, i + 7))
  }

  return groups
}

export {
  eventDateKey,
  formatDateTime,
  formatDate,
  formatTime,
  groupDatesByWeek,
  dateRangeToApiParams,
}
