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

export { formatDateTime }
