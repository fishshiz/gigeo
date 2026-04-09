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
            className="box-border h-10 w-10 cursor-pointer rounded-xl bg-slate-400 p-2 text-center text-sm/6 leading-none font-thin"
            onClick={() => onSelect(day)}
            style={{
              background:
                activeDateId && activeDateId === day
                  ? "var(--color-green-400)"
                  : "var(--color-slate-400)",
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
