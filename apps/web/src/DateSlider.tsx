import { useDateFormatter } from "react-aria"
import { parseDate, getLocalTimeZone } from "@internationalized/date"
import { useEffect, useRef } from "react"
import { groupDatesByWeek } from "./lib/dates"

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
  const groupedDates = groupDatesByWeek(dates)
  const groupRefs = useRef(new Map<number, HTMLDivElement>())

  useEffect(() => {
    if (!activeDateId) return
    const groupIndex = groupedDates.findIndex((group) =>
      group.includes(activeDateId)
    )
    if (groupIndex === -1) return

    groupRefs.current.get(groupIndex)?.scrollIntoView({
      inline: "nearest",
      block: "nearest",
      behavior: "smooth",
    })
  }, [activeDateId, groupedDates])

  return (
    <div className="flex w-full items-center">
      <ul className="flex h-full w-full snap-x justify-around gap-2 overflow-x-scroll">
        {groupedDates.map((days, groupIdx) => (
          <div
            key={days[0] ?? groupIdx}
            ref={(node) => {
              if (node) groupRefs.current.set(groupIdx, node)
              else groupRefs.current.delete(groupIdx)
            }}
            className="flex w-full flex-[0_0_100%] snap-center items-center gap-2"
          >
            {days.map((day) => {
              const isActive = Boolean(activeDateId && activeDateId === day)
              return (
                <li key={day} className="snap-center">
                  <button
                    type="button"
                    aria-current={isActive ? "date" : undefined}
                    className={
                      "box-border min-w-14 cursor-pointer rounded-xl p-2 max-md:py-2.5! text-center text-xs leading-none transition-colors duration-150 outline-none focus-visible:ring-2 focus-visible:ring-(--color-ivory-600) focus-visible:ring-offset-1 " +
                      (isActive
                        ? "bg-(--color-ivory-600)"
                        : "bg-slate-400 hover:bg-slate-500 dark:bg-(--color-surface-dark-400) dark:hover:bg-(--color-surface-dark-300)")
                    }
                    onClick={() => onSelect(day)}
                  >
                    <span className="block font-semibold text-white">
                      {dayFormatter.format(
                        parseDate(day).toDate(getLocalTimeZone())
                      )}
                    </span>
                    <span className="text-white/90">
                      {dateFormatter.format(
                        parseDate(day).toDate(getLocalTimeZone())
                      )}
                    </span>
                  </button>
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
