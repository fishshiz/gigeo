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
  base: "flex font-sans max-w-full",
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
  // `group/tablist` -- react-aria-components puts `data-orientation` on
  // this element, not on each `Tab` (confirmed live: a Tab's own rendered
  // DOM node carries no such attribute) -- so `indicatorStyles.bar` reads
  // it from here via a *named* group rather than the unnamed `group` each
  // Tab already claims for its own `group-disabled:` styling.
  base: "group/tablist flex max-w-full p-1 -m-1 overflow-x-auto overflow-y-clip [scrollbar-width:none]",
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
  // `isolate` keeps the SelectionIndicator (a -z-10 sibling) from painting
  // below page content outside this tab -- it's a solid accent shape
  // sitting behind the icon/label, not a blend trick, so it needs its own
  // stacking context to stay contained rather than any special compositing.
  base: "group relative isolate flex items-center cursor-default rounded-full px-3 py-1.5 text-sm font-medium transition forced-color-adjust-none [-webkit-tap-highlight-color:transparent]",
  variants: {
    isDisabled: {
      true: "text-neutral-200 dark:text-neutral-600 forced-colors:text-[GrayText] selected:text-white dark:selected:text-neutral-500 forced-colors:selected:text-[HighlightText] selected:bg-neutral-200 dark:selected:bg-neutral-600 forced-colors:selected:bg-[GrayText]",
    },
    // "pill" (default): the selected tab gets a full accent-filled
    // background, so its own label/icon switch to the on-accent contrast
    // color. "bar": no fill -- see `indicatorStyles` below -- so the label/
    // icon instead pick up the accent color directly, the same way an
    // underlined/bar-indicated tab would elsewhere.
    variant: {
      pill: "selected:text-(--text-on-accent)",
      bar: "selected:text-(--accent-bg)",
    },
  },
  defaultVariants: {
    variant: "pill",
  },
})

// A solid dash rather than `indicatorStyles.pill`'s full bubble --
// right-aligned (a vertical dash hugging the sidebar's own divider) when
// the tabs are laid out vertically, bottom-aligned (a horizontal dash
// underlining the tab) when horizontal. `group-data-[orientation=...]/
// tablist` reads react-aria-components' own `data-orientation` attribute
// via the named group `tabListStyles` puts on the *TabList* -- confirmed
// live that a `Tab` itself carries no such attribute, only its TabList
// does -- so one Tab template automatically renders the right dash
// orientation for both DrawerWrapper's desktop sidebar and mobile bottom
// bar without needing a separate prop for it. Centered via inset+auto-
// margins rather than a translate utility, since SelectionIndicator
// already owns the `translate` CSS property itself (see
// `motion-safe:transition-[translate,width,height]`) to animate between
// tabs -- a Tailwind translate utility here would fight that.
const indicatorStyles = {
  pill: "absolute top-0 left-0 -z-10 h-full w-full rounded-full bg-(--accent-bg) group-disabled:bg-neutral-200 motion-safe:transition-[translate,width,height] dark:group-disabled:bg-neutral-600",
  bar: "absolute -z-10 rounded-full bg-(--accent-bg) group-disabled:bg-neutral-300 motion-safe:transition-[translate,width,height] dark:group-disabled:bg-neutral-600 group-data-[orientation=horizontal]/tablist:inset-x-0 group-data-[orientation=horizontal]/tablist:bottom-0 group-data-[orientation=horizontal]/tablist:mx-auto group-data-[orientation=horizontal]/tablist:h-[3px] group-data-[orientation=horizontal]/tablist:w-12 group-data-[orientation=vertical]/tablist:inset-y-0 group-data-[orientation=vertical]/tablist:right-0 group-data-[orientation=vertical]/tablist:my-auto group-data-[orientation=vertical]/tablist:h-6 group-data-[orientation=vertical]/tablist:w-[3px]",
} as const

export function Tab(
  props: TabProps & { variant?: keyof typeof indicatorStyles }
) {
  const { variant = "pill", ...tabOnlyProps } = props
  return (
    <RACTab
      {...tabOnlyProps}
      className={composeRenderProps(props.className, (className, renderProps) =>
        tabProps({ ...renderProps, variant, className })
      )}
    >
      {composeRenderProps(props.children, (children) => (
        <>
          {children}
          <SelectionIndicator className={indicatorStyles[variant]} />
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
