import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
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

const groupDatesByWeek = (dates: string[]): string[][] => {
  const groups: string[][] = []

  for (let i = 0; i < dates.length; i += 7) {
    groups.push(dates.slice(i, i + 7))
  }

  return groups
}

export { formatDateTime, formatDate, formatTime, groupDatesByWeek }
