import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
  type CalendarDate,
} from "@internationalized/date"

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

const formatDateTime = (date: string) => {
  if (!hasTimeComponent(date)) return formatDate(date)

  const parsedDate = parseAbsolute(date, "UTC")

  const dateFormatter = new DateFormatter("en-US", {
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone: getLocalTimeZone(), // or a specific IANA tz
  })
  return dateFormatter.format(parsedDate.toDate())
}

const formatTime = (dateString: string) => {
  if (!hasTimeComponent(dateString)) return null

  const parsedDate = parseAbsolute(dateString, "UTC")

  const formatter = new DateFormatter("en-US", {
    hour: "numeric",
    minute: "2-digit",
    hour12: true,
    timeZone: getLocalTimeZone(),
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
  formatDateTime,
  formatDate,
  formatTime,
  groupDatesByWeek,
  dateRangeToApiParams,
}
