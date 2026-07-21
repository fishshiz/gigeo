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
  const dateFormatter = useDateFormatter({
    month: "short",
    day: "numeric",
  })
  const dayFormatter = useDateFormatter({
    weekday: "short",
  })
  const groupedDates: string[][] = dates.reduce(
    (acc: string[][], curr: string, idx: number) => {
      if (idx % 7 === 0) {
        acc.push([curr])
      } else if (idx < 7) {
        console.log(acc, idx, curr)
        acc[(idx % 7) - idx].push(curr)
      } else if (acc[idx % 7]) {
        acc[idx % 7].push(curr)
      } else {
        console.log("else", acc, idx, curr)
      }
      return acc
    },
    []
  )
  console.log(groupedDates)

  return (
    <div className="flex w-full items-center">
      <ul className="flex h-full w-full snap-x justify-around gap-2 overflow-x-scroll">
        {groupedDates.map((days) => (
          <div className="flex w-full flex-[0_0_100%] snap-center items-center gap-2">
            {days.map((day) => (
              <li
                className="box-border cursor-pointer snap-center rounded-xl bg-slate-400 p-2 text-center text-xs leading-none"
                onClick={() => onSelect(day)}
                style={{
                  background:
                    activeDateId && activeDateId === day
                      ? "var(--color-green-400)"
                      : "var(--color-slate-400)",
                }}
              >
                <h4 className="font-semibold text-white">
                  {dayFormatter.format(
                    parseDate(day).toDate(getLocalTimeZone())
                  )}
                </h4>
                <span>
                  {dateFormatter.format(
                    parseDate(day).toDate(getLocalTimeZone())
                  )}
                </span>
              </li>
            ))}
          </div>
        ))}
      </ul>
    </div>
  )
}
export { DateSlider }
