"use client"
import {
  GridList as AriaGridList,
  GridListItem as AriaGridListItem,
  GridListHeader as AriaGridListHeader,
  Button,
  composeRenderProps,
} from "react-aria-components"
import type { GridListItemProps, GridListProps } from "react-aria-components"
import { tv } from "tailwind-variants"
import { Checkbox } from "@workspace/ui/components/ui/Checkbox"
import {
  composeTailwindRenderProps,
  focusRing,
} from "@workspace/ui/lib/react-aria-utils"
import type { HTMLAttributes } from "react"
import { twMerge } from "tailwind-merge"

export function GridList<T extends object>({
  children,
  ...props
}: GridListProps<T>) {
  return (
    <AriaGridList
      {...props}
      className={composeTailwindRenderProps(
        props.className,
        "relative w-[200px] overflow-auto rounded-lg border border-neutral-300 bg-white font-sans empty:flex empty:items-center empty:justify-center empty:text-sm empty:italic dark:border-neutral-700 dark:bg-neutral-900"
      )}
    >
      {children}
    </AriaGridList>
  )
}

const itemStyles = tv({
  extend: focusRing,
  base: "relative flex gap-3 cursor-default select-none py-2 px-3 text-sm text-neutral-900 dark:text-neutral-200 border-t dark:border-t-neutral-700 border-transparent first:border-t-0 first:rounded-t-lg last:rounded-b-lg last:mb-0 -outline-offset-2",
  variants: {
    isSelected: {
      false:
        "hover:bg-neutral-100 pressed:bg-neutral-100 dark:hover:bg-neutral-700/60 dark:pressed:bg-neutral-700/60",
      true: "bg-blue-100 dark:bg-blue-700/30 hover:bg-blue-200 pressed:bg-blue-200 dark:hover:bg-blue-700/40 dark:pressed:bg-blue-700/40 border-y-blue-200 dark:border-y-blue-900 z-20",
    },
    isDisabled: {
      true: "text-neutral-300 dark:text-neutral-600 forced-colors:text-[GrayText] z-10",
    },
  },
})

export function GridListItem({ children, ...props }: GridListItemProps) {
  const textValue = typeof children === "string" ? children : undefined
  return (
    <AriaGridListItem textValue={textValue} {...props} className={itemStyles}>
      {composeRenderProps(
        children,
        (children, { selectionMode, selectionBehavior, allowsDragging }) => (
          <>
            {/* Add elements for drag and drop and selection. */}
            {allowsDragging && <Button slot="drag">≡</Button>}
            {selectionMode !== "none" && selectionBehavior === "toggle" && (
              <Checkbox slot="selection" />
            )}
            {children}
          </>
        )
      )}
    </AriaGridListItem>
  )
}

export function GridListHeader({
  children,
  ...props
}: HTMLAttributes<HTMLElement>) {
  return (
    <AriaGridListHeader
      {...props}
      className={twMerge(
        "z-10 -mt-px border-y border-y-neutral-200 bg-neutral-100/60 px-4 py-1 text-sm font-semibold text-neutral-500 backdrop-blur-md supports-[-moz-appearance:none]:bg-neutral-100 dark:border-y-neutral-700 dark:bg-neutral-700/60 dark:text-neutral-300",
        props.className
      )}
    >
      {children}
    </AriaGridListHeader>
  )
}
