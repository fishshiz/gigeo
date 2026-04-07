import { Button } from "@workspace/ui/components/ui/Button"
import { ChevronLeftCircleIcon, ChevronRightCircleIcon } from "lucide-react"
import { useState, useEffect } from "react"
import type { DateRange, DateValue } from "react-aria"
const DateSlider = ({
  dateRange,
  onSelect,
}: {
  dateRange: DateRange
  onSelect: (date: DateValue) => void
}) => {
  console.log(dateRange)
  const [days, setDays] = useState<DateValue[]>([])
  const getDaysInRange = () => {
    let current = dateRange.start
    const dates: DateValue[] = []

    while (current.compare(dateRange.end) <= 0) {
      dates.push(current)
      current = current.add({ days: 1 })
    }
    setDays(dates)
  }

  useEffect(() => {
    getDaysInRange()
  }, [dateRange])
  return (
    <div className="flex w-full items-center">
      <Button>
        <ChevronLeftCircleIcon />
      </Button>
      <ul className="flex w-full flex-1 justify-around">
        {days.map((day) => (
          <li
            className="h-6 w-6 rounded-xl bg-slate-400 text-center"
            onClick={() => onSelect(day)}
          >
            {day.day}
          </li>
        ))}
      </ul>
      <Button>
        <ChevronRightCircleIcon />
      </Button>
    </div>
  )
}
export { DateSlider }
