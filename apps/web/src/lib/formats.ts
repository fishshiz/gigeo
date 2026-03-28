import {
  parseAbsolute,
  parseDate,
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

const formatDate = (date: string) => {
  const hasTime = date.includes("T")
  const parsedDate = hasTime ? parseAbsolute(date, "UTC") : parseDate(date)
  const dateFormatter = new DateFormatter("en-US", {
    month: "long",
    day: "numeric",
  })
  return dateFormatter.format(parsedDate.toDate(getLocalTimeZone()))
}

export { formatDateTime, formatDate }
