"use client"
import { CalendarIcon } from "lucide-react"
import React from "react"
import {
  DateRangePicker as AriaDateRangePicker,
  DateRangePickerProps as AriaDateRangePickerProps,
  DateValue,
  ValidationResult,
} from "react-aria-components"
import {
  Description,
  FieldError,
  FieldGroup,
  Label,
} from "@workspace/ui/components/ui/Field"
import { Popover } from "@workspace/ui/components/ui/Popover"
import { RangeCalendar } from "@workspace/ui/components/ui/RangeCalendar"
import { composeTailwindRenderProps } from "@workspace/ui/lib/react-aria-utils"
import { FieldButton } from "@workspace/ui/components/ui/FieldButton"
import { useDateFormatter } from "react-aria"
import { isSameMonth, isSameDay } from "@internationalized/date"

export interface DateRangePickerProps<
  T extends DateValue,
> extends AriaDateRangePickerProps<T> {
  label?: string
  description?: string
  errorMessage?: string | ((validation: ValidationResult) => string)
}

export function DateRangePicker<T extends DateValue>({
  label,
  description,
  errorMessage,
  ...props
}: DateRangePickerProps<T>) {
  console.log(props)
  const monthDayFormat = useDateFormatter({
    month: "long",
    day: "numeric",
  })

  const dayFormat = useDateFormatter({
    day: "numeric",
  })

  const start = props.value?.start
  const end = props.value?.end
  return (
    <AriaDateRangePicker
      {...props}
      className={composeTailwindRenderProps(
        props.className,
        "group flex max-w-full flex-col gap-1 font-sans"
      )}
    >
      {label && <Label>{label}</Label>}
      <FieldGroup className="disabled:cursor-default">
        <FieldButton className="mr-1 w-full cursor-pointer outline-offset-0">
          <div className="flex w-fit flex-1 items-center overflow-x-auto overflow-y-clip [scrollbar-width:none]">
            <span className="w-max ps-3 pe-2 text-sm">
              {monthDayFormat.format(start?.toDate())}
            </span>
            {!isSameDay(start, end) && (
              <>
                <span
                  aria-hidden="true"
                  className="text-neutral-800 group-disabled:text-neutral-200 dark:text-neutral-200 dark:group-disabled:text-neutral-600 forced-colors:text-[ButtonText] forced-colors:group-disabled:text-[GrayText]"
                >
                  –
                </span>
                <span className="w-max flex-1 ps-2 pe-3 text-sm">
                  {isSameMonth(start, end)
                    ? dayFormat.format(end?.toDate())
                    : monthDayFormat.format(end?.toDate())}
                </span>
              </>
            )}
          </div>
          <CalendarIcon aria-hidden className="h-4 w-4" />
        </FieldButton>
      </FieldGroup>
      {description && <Description>{description}</Description>}
      <FieldError>{errorMessage}</FieldError>
      <Popover className="p-2">
        <RangeCalendar />
      </Popover>
    </AriaDateRangePicker>
  )
}
