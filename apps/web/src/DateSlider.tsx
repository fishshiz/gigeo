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
        acc[(idx % 7) - idx].push(curr)
      } else if (acc[idx % 7]) {
        acc[idx % 7].push(curr)
      }
      return acc
    },
    []
  )

  return (
    <div className="flex w-full items-center">
      <ul className="flex h-full w-full snap-x justify-around gap-2 overflow-x-scroll">
        {groupedDates.map((days, groupIdx) => (
          <div
            key={days[0] ?? groupIdx}
            className="flex w-full flex-[0_0_100%] snap-center items-center gap-2"
          >
            {days.map((day) => {
              const isActive = Boolean(activeDateId && activeDateId === day)
              return (
                <li
                  key={day}
                  className={
                    "box-border min-w-14 cursor-pointer snap-center rounded-xl p-2 text-center text-xs leading-none transition " +
                    (isActive
                      ? "bg-(--color-ivory-600)"
                      : "bg-slate-400 hover:bg-slate-500 dark:bg-(--color-surface-dark-400) dark:hover:bg-(--color-surface-dark-300)")
                  }
                  onClick={() => onSelect(day)}
                >
                  <h4 className="font-semibold text-white">
                    {dayFormatter.format(
                      parseDate(day).toDate(getLocalTimeZone())
                    )}
                  </h4>
                  <span className="text-white/90">
                    {dateFormatter.format(
                      parseDate(day).toDate(getLocalTimeZone())
                    )}
                  </span>
                </li>
              )
            })}
          </div>
        ))}
      </ul>
    </div>
  )
}
export { DateSlider }
