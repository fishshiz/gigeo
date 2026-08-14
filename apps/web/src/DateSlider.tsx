import { useDateFormatter, useLocale, type RangeValue } from "react-aria"
import { getLocalTimeZone, type CalendarDate } from "@internationalized/date"
import { useEffect, useMemo, useRef } from "react"
import { motion, useReducedMotion } from "motion/react"
import { ChevronLeftIcon, ChevronRightIcon } from "lucide-react"
import { buildCalendarWeeks, dayCellState, type DayCellState } from "./lib/dates"
import { useCarouselIndex } from "./hooks/useCarouselIndex"

// Distinct from MOTION_DURATION/MOTION_EASE (@workspace/ui/lib/motion) --
// those are duration/ease pairs for size/opacity transitions; this is a
// spring, for the same snappy-settle feel already established by the
// bottom sheet's SNAP_SPRING (Drawer.tsx), just lighter since this moves a
// small pill rather than a whole sheet.
const ACTIVE_PILL_SPRING = { type: "spring", stiffness: 500, damping: 32 } as const

const dayCellClass = (state: DayCellState, isActive: boolean) => {
  if (isActive) return "text-white font-semibold"
  switch (state) {
    case "out-of-range":
      return "text-gray-300 dark:text-gray-700"
    case "empty":
      return "text-gray-700 dark:text-gray-300"
    case "has-events":
      return "font-medium text-gray-900 dark:text-gray-100"
  }
}

const DateSlider = ({
  dateRange,
  eventDates,
  onSelect,
  activeDateId,
}: {
  dateRange: RangeValue<CalendarDate>
  eventDates: ReadonlySet<string>
  onSelect: (date: string) => void
  activeDateId: string | null
}) => {
  const { locale } = useLocale()
  const timeZone = getLocalTimeZone()
  const shouldReduceMotion = useReducedMotion()
  const weekdayFormatter = useDateFormatter({ weekday: "narrow" })
  const cellLabelFormatter = useDateFormatter({
    weekday: "long",
    month: "long",
    day: "numeric",
  })

  const weeks = useMemo(
    () => buildCalendarWeeks(dateRange.start, dateRange.end, locale),
    [dateRange.start, dateRange.end, locale]
  )
  const weekdayLabels = useMemo(
    () => weeks[0]?.map((day) => weekdayFormatter.format(day.toDate(timeZone))) ?? [],
    [weeks, weekdayFormatter, timeZone]
  )

  const containerRef = useRef<HTMLUListElement | null>(null)
  const { activeIndex: visibleWeekIndex, scrollToIndex } =
    useCarouselIndex(containerRef)

  useEffect(() => {
    if (!activeDateId) return
    const weekIndex = weeks.findIndex((week) =>
      week.some((day) => day.toString() === activeDateId)
    )
    if (weekIndex === -1) return
    scrollToIndex(weekIndex)
  }, [activeDateId, weeks, scrollToIndex])

  return (
    <div className="flex w-full flex-col items-center gap-1">
      <div className="flex w-full items-center justify-around px-8">
        {weekdayLabels.map((label, i) => (
          <span
            key={i}
            className="w-8 text-center text-[11px] font-medium text-gray-500 dark:text-gray-400"
          >
            {label}
          </span>
        ))}
      </div>

      <div className="flex w-full items-center gap-1">
        <button
          type="button"
          aria-label="Previous week"
          disabled={visibleWeekIndex <= 0}
          onClick={() => scrollToIndex(visibleWeekIndex - 1)}
          className="shrink-0 cursor-pointer rounded-full p-1 text-gray-400 transition-colors hover:bg-slate-100 hover:text-gray-600 disabled:pointer-events-none disabled:opacity-0 dark:hover:bg-(--color-surface-dark-300) dark:hover:text-gray-200"
        >
          <ChevronLeftIcon aria-hidden className="h-4 w-4" />
        </button>

        <ul
          ref={containerRef}
          className="flex w-full snap-x snap-mandatory overflow-x-auto [scrollbar-width:none]"
        >
          {weeks.map((week) => (
            <li
              key={week[0].toString()}
              className="flex w-full flex-[0_0_100%] snap-center justify-around"
            >
              {week.map((day) => {
                const key = day.toString()
                const state = dayCellState(
                  day,
                  dateRange.start,
                  dateRange.end,
                  eventDates
                )
                const isActive = activeDateId === key
                const label = cellLabelFormatter.format(day.toDate(timeZone))

                if (state === "out-of-range") {
                  return (
                    <div
                      key={key}
                      aria-hidden="true"
                      className="flex w-8 flex-col items-center gap-0.5 p-1"
                    >
                      <span className={`text-sm ${dayCellClass(state, false)}`}>
                        {day.day}
                      </span>
                      <span className="block h-1 w-1" />
                    </div>
                  )
                }

                return (
                  <button
                    key={key}
                    type="button"
                    disabled={state !== "has-events"}
                    aria-current={isActive ? "date" : undefined}
                    aria-label={label}
                    title={label}
                    onClick={() => onSelect(key)}
                    className="flex w-8 cursor-pointer flex-col items-center gap-0.5 p-1 outline-none disabled:cursor-default focus-visible:ring-2 focus-visible:ring-(--color-ivory-600) focus-visible:ring-offset-1"
                  >
                    <span className="relative flex h-7 w-7 items-center justify-center">
                      {isActive && (
                        <motion.div
                          layoutId="date-slider-active-pill"
                          className="absolute inset-0 rounded-full bg-(--color-ivory-600)"
                          transition={
                            shouldReduceMotion
                              ? { duration: 0 }
                              : ACTIVE_PILL_SPRING
                          }
                        />
                      )}
                      <span
                        className={`relative z-10 text-sm ${dayCellClass(state, isActive)}`}
                      >
                        {day.day}
                      </span>
                    </span>
                    <span
                      className={
                        "block h-1 w-1 rounded-full bg-(--color-ivory-600) " +
                        (state === "has-events" ? "opacity-100" : "opacity-0")
                      }
                    />
                  </button>
                )
              })}
            </li>
          ))}
        </ul>

        <button
          type="button"
          aria-label="Next week"
          disabled={visibleWeekIndex >= weeks.length - 1}
          onClick={() => scrollToIndex(visibleWeekIndex + 1)}
          className="shrink-0 cursor-pointer rounded-full p-1 text-gray-400 transition-colors hover:bg-slate-100 hover:text-gray-600 disabled:pointer-events-none disabled:opacity-0 dark:hover:bg-(--color-surface-dark-300) dark:hover:text-gray-200"
        >
          <ChevronRightIcon aria-hidden className="h-4 w-4" />
        </button>
      </div>
    </div>
  )
}

export { DateSlider }
