import {
  parseAbsolute,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"

const formatDateTime = (date: string) => {
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

const groupDatesByWeek = (dates: string[]): string[][] => {
  const groups: string[][] = []

  for (let i = 0; i < dates.length; i += 7) {
    groups.push(dates.slice(i, i + 7))
  }

  return groups
}

export { formatDateTime, formatDate, groupDatesByWeek }
