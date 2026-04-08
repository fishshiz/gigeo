import { Button } from "@workspace/ui/components/ui/Button"
import { ChevronLeftCircleIcon, ChevronRightCircleIcon } from "lucide-react"
import { useDateFormatter } from "react-aria"
import { parseDate, getLocalTimeZone } from "@internationalized/date"
const DateSlider = ({
  dates,
  onSelect,
  activeDateId,
}: {
  dates: string[]
  onSelect: (date: string) => void
  activeDateId: string | null
}) => {
  const dateFormatter = useDateFormatter({ month: "short", day: "numeric" })

  return (
    <div className="flex w-full items-center">
      <Button>
        <ChevronLeftCircleIcon />
      </Button>
      <ul className="flex w-full flex-1 justify-around">
        {dates.map((day) => (
          <li
            className="h-6 w-6 rounded-xl bg-slate-400 text-center"
            onClick={() => onSelect(day)}
            style={{
              background:
                activeDateId && activeDateId === day ? "red" : "green",
            }}
          >
            {dateFormatter.format(parseDate(day).toDate(getLocalTimeZone()))}
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
