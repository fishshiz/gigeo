"use client"
import {
  Link as AriaLink,
  type LinkProps as AriaLinkProps,
  composeRenderProps,
} from "react-aria-components"
import { tv } from "tailwind-variants"
import { focusRing } from "@workspace/ui/lib/react-aria-utils"

interface LinkProps extends AriaLinkProps {
  variant?: "primary" | "secondary" | "button"
}

const styles = tv({
  extend: focusRing,
  base: "underline disabled:no-underline disabled:cursor-default forced-colors:disabled:text-[GrayText] transition rounded-xs [-webkit-tap-highlight-color:transparent]",
  variants: {
    variant: {
      primary:
        "text-blue-600 dark:text-blue-500 underline decoration-blue-600/60 hover:decoration-blue-600 dark:decoration-blue-500/60 dark:hover:decoration-blue-500",
      secondary:
        "text-neutral-700 dark:text-neutral-300 underline decoration-neutral-700/50 hover:decoration-neutral-700 dark:decoration-neutral-300/70 dark:hover:decoration-neutral-300",
      button:
        "relative inline-flex items-center justify-center gap-2 border border-transparent dark:border-white/10 h-9 box-border px-3.5 py-0 [&:has(>svg:only-child)]:px-0 [&:has(>svg:only-child)]:h-8 [&:has(>svg:only-child)]:w-8 font-sans text-sm text-center transition rounded-lg cursor-default [-webkit-tap-highlight-color:transparent] bg-blue-600 hover:bg-blue-700 pressed:bg-blue-800 text-white",
    },
  },
  defaultVariants: {
    variant: "primary",
  },
})

export function Link(props: LinkProps) {
  return (
    <AriaLink
      {...props}
      className={composeRenderProps(props.className, (className, renderProps) =>
        styles({ ...renderProps, className, variant: props.variant })
      )}
    />
  )
}
