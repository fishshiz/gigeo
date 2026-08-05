"use client"
import {
  Tab as RACTab,
  TabList as RACTabList,
  TabPanels as RACTabPanels,
  TabPanel as RACTabPanel,
  Tabs as RACTabs,
  SelectionIndicator,
  composeRenderProps,
} from "react-aria-components"
import type {
  TabListProps,
  TabPanelProps,
  TabPanelsProps,
  TabProps,
  TabsProps,
} from "react-aria-components"
import { tv } from "tailwind-variants"
import { focusRing } from "@workspace/ui/lib/react-aria-utils"
import { twMerge } from "tailwind-merge"

const tabsStyles = tv({
  base: "flex gap-4 font-sans max-w-full",
  variants: {
    orientation: {
      horizontal: "flex-col",
      vertical: "flex-row",
    },
  },
})

export function Tabs(props: TabsProps) {
  return (
    <RACTabs
      {...props}
      className={composeRenderProps(props.className, (className, renderProps) =>
        tabsStyles({ ...renderProps, className })
      )}
    />
  )
}

const tabListStyles = tv({
  base: "flex max-w-full p-1 -m-1 overflow-x-auto overflow-y-clip [scrollbar-width:none]",
  variants: {
    orientation: {
      horizontal: "flex-row",
      vertical: "flex-col items-start",
    },
  },
})

export function TabList<T extends object>(props: TabListProps<T>) {
  return (
    <RACTabList
      {...props}
      className={composeRenderProps(props.className, (className, renderProps) =>
        tabListStyles({ ...renderProps, className })
      )}
    />
  )
}

const tabProps = tv({
  extend: focusRing,
  // `isolate` scopes the SelectionIndicator's mix-blend-difference below to
  // just this tab's own stacking context. Without it, the blend composites
  // against whatever's behind the tab in the page's shared stacking context
  // instead of just this tab's own icon/label — on a dark background that
  // reads as a solid-black circle with the icon and label blended away
  // entirely, rather than the intended inverted-color pill.
  //
  // `selected:text-white` forces the icon/label to a known, fixed color
  // before the blend runs. The indicator's own background is always white
  // and, blended against the transparent space around the icon/label, always
  // composites out to white too — so the indicator itself never varies by
  // theme. Left to inherit the normal (theme-dependent) text color, the
  // difference-blend is far weaker in light mode: OKLCH's perceptually-even
  // lightness scale isn't linear in sRGB, so light mode's dark text (~35/255)
  // blends to a washed-out light gray (~220/255) against the white pill,
  // while dark mode's light text (~250/255) happens to blend to strong
  // near-black — the same trick, but only reliably readable in one theme.
  // Pinning to white first makes every theme diff from the same value dark
  // mode already gets for free: near-maximum, near-black marks on the pill.
  base: "group relative isolate flex items-center cursor-default rounded-full px-3 py-1.5 text-sm font-medium transition selected:text-white forced-color-adjust-none [-webkit-tap-highlight-color:transparent]",
  variants: {
    isDisabled: {
      true: "text-neutral-200 dark:text-neutral-600 forced-colors:text-[GrayText] selected:text-white dark:selected:text-neutral-500 forced-colors:selected:text-[HighlightText] selected:bg-neutral-200 dark:selected:bg-neutral-600 forced-colors:selected:bg-[GrayText]",
    },
  },
})

export function Tab(props: TabProps) {
  return (
    <RACTab
      {...props}
      className={composeRenderProps(props.className, (className, renderProps) =>
        tabProps({ ...renderProps, className })
      )}
    >
      {composeRenderProps(props.children, (children) => (
        <>
          {children}
          <SelectionIndicator className="absolute top-0 left-0 z-10 h-full w-full rounded-full bg-white mix-blend-difference group-disabled:-z-1 group-disabled:bg-neutral-400 group-disabled:mix-blend-normal motion-safe:transition-[translate,width,height] group-disabled:dark:bg-neutral-600" />
        </>
      ))}
    </RACTab>
  )
}

export function TabPanels<T extends object>(props: TabPanelsProps<T>) {
  return (
    <RACTabPanels
      {...props}
      className={twMerge(
        "flex h-full min-h-0 min-w-0 flex-1 flex-col overflow-clip",
        props.className
      )}
    />
  )
}

const tabPanelStyles = tv({
  extend: focusRing,
  base: "flex-1 min-h-0 box-border text-sm text-neutral-900 dark:text-neutral-100 transition entering:opacity-0 exiting:opacity-0 exiting:absolute exiting:top-0 exiting:left-0 exiting:w-full",
})

export function TabPanel(props: TabPanelProps) {
  return (
    <RACTabPanel
      {...props}
      className={composeRenderProps(props.className, (className, renderProps) =>
        tabPanelStyles({ ...renderProps, className })
      )}
    />
  )
}
