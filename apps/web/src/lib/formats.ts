import {
  parseAbsolute,
  parseDate,
  DateFormatter,
  getLocalTimeZone,
} from "@internationalized/date"
import { useDateFormatter } from "react-aria"

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
  // 1. Parse YYYY-MM-DD
  const [year, month, day] = dateString.split("-").map(Number)
  // Create a Date object set to UTC to avoid timezone shifts
  const date = new Date(Date.UTC(year, month - 1, day))

  // 2. Use React Aria/Intl formatter
  // Note: 'date' is a native Date object, which useDateFormatter accepts
  let formatter = useDateFormatter({
    year: "numeric",
    month: "long",
    day: "numeric",
    timeZone: "UTC", // Important for matching the input string
  })

  return formatter.format(date)
}

export { formatDateTime, formatDate }
