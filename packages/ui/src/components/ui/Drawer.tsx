"use client"

import {
  AnimatePresence,
  animate,
  motion,
  useDragControls,
  useMotionValue,
  useReducedMotion,
} from "motion/react"
import { use, Children, cloneElement, useEffect, useRef } from "react"
import type { Ref } from "react"
import type {
  DialogProps,
  DialogTriggerProps,
  HeadingProps,
  ModalOverlayProps,
  TextProps,
} from "react-aria-components"
import {
  Button as ButtonPrimitive,
  Dialog,
  DialogTrigger,
  Heading,
  OverlayTriggerStateContext,
  Text,
} from "react-aria-components"
import { twMerge } from "tailwind-merge"
import { Button, type ButtonProps } from "./Button"

type PassThroughProps = {
  children: React.ReactNode
  [key: string]: unknown // allow any additional props
}
const PassThrough = ({ children, ...props }: PassThroughProps) => {
  const child = Children.only(children) // throws if not exactly 1 child
  return cloneElement(child as React.ReactElement, { ...props })
}

const DrawerRoot = motion.create(PassThrough)

const Drawer = (props: DialogTriggerProps) => <DialogTrigger {...props} />

interface DrawerContentProps
  extends
    Omit<ModalOverlayProps, "className" | "children" | "isDismissable">,
    Pick<
      DialogProps,
      "aria-label" | "aria-labelledby" | "role" | "children" | "className"
    > {
  isFloat?: boolean
  className?: string
  side?: "top" | "bottom" | "left" | "right"
  notch?: boolean
  closeDrawer: () => void
  /** Ascending pixel heights for a draggable multi-snap bottom sheet
   * (e.g. [peekPx, halfPx, fullPx]) -- side="bottom" only. Omit for the
   * regular binary open/close sheet every other caller uses; when
   * omitted, drag/animate behavior is byte-identical to before this
   * feature existed. */
  snapPointsPx?: number[]
  /** Index into snapPointsPx for the currently-settled snap. */
  activeSnapIndex?: number
  /** Fired when a drag gesture resolves to a snap index (possibly
   * unchanged from activeSnapIndex -- the sheet still re-settles to the
   * exact target pixel position either way). */
  onSnapChange?: (index: number) => void
}

// Matches the bounce feel already established by dragTransition below,
// expressed as a real spring transition (not dragTransition, which only
// governs live elastic recoil *during* a drag, not this post-release
// settle) -- used both by the settle effect and directly in onDragEnd.
const SNAP_SPRING = { type: "spring", stiffness: 600, damping: 20 } as const
const FLING_VELOCITY_THRESHOLD = 500 // px/s
const VELOCITY_PROJECTION_SECONDS = 0.15

const DrawerContent = ({
  side = "bottom",
  isFloat = false,
  notch = true,
  closeDrawer = () => {},
  children,
  className,
  snapPointsPx,
  activeSnapIndex,
  onSnapChange,
  ...props
}: DrawerContentProps) => {
  const state = use(OverlayTriggerStateContext)!
  const dragControls = useDragControls()
  const shouldReduceMotion = useReducedMotion()
  const offscreen = {
    x: side === "left" ? "-100%" : side === "right" ? "100%" : 0,
    y: side === "top" ? "-100%" : side === "bottom" ? "100%" : 0,
  }

  const isMultiSnapBottom =
    side === "bottom" &&
    !!snapPointsPx &&
    snapPointsPx.length > 1 &&
    activeSnapIndex !== undefined

  // Downward-translate targets, one per snap point -- the smallest height
  // (peek) is the *largest* y (most pushed down); the tallest (full) is
  // always y=0. Only meaningful when isMultiSnapBottom, but computed
  // unconditionally since hooks below need a stable reference either way.
  const snapYs = (snapPointsPx ?? []).map(
    (h) => (snapPointsPx ?? [])[(snapPointsPx ?? []).length - 1] - h
  )

  // Controlled motion value: bound to the element via `style` below so a
  // live drag gesture and this settle effect / onDragEnd's imperative
  // animate() both read and write the exact same value, with no jump
  // between "being dragged" and "being programmatically animated".
  const ySpring = useMotionValue(
    isMultiSnapBottom ? snapYs[activeSnapIndex!] : 0
  )

  // Settles ySpring whenever activeSnapIndex changes from *outside* a drag
  // (e.g. DrawerWrapper's auto-transition effects calling setSnapPoint).
  // The drag-driven case settles immediately inside onDragEnd itself,
  // below -- this effect is what covers the non-drag path. Skips the
  // spring (jumps instantly) when only snapPointsPx changed -- e.g. a
  // viewport resize -- so that alone never produces a visible bounce.
  const prevSnapPointsPxRef = useRef(snapPointsPx)
  useEffect(() => {
    if (!isMultiSnapBottom || activeSnapIndex === undefined) return
    const onlyPointsChanged = prevSnapPointsPxRef.current !== snapPointsPx
    prevSnapPointsPxRef.current = snapPointsPx
    const controls = animate(
      ySpring,
      snapYs[activeSnapIndex],
      shouldReduceMotion || onlyPointsChanged ? { duration: 0 } : SNAP_SPRING
    )
    return () => controls.stop()
    // snapYs is derived fresh from snapPointsPx every render (new array
    // identity each time) -- depending on snapPointsPx itself is the
    // correct, stable trigger.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isMultiSnapBottom, activeSnapIndex, snapPointsPx, shouldReduceMotion])

  const handleBinaryDragEnd = (
    _: React.MouseEvent<HTMLDivElement>,
    {
      offset,
      velocity,
    }: {
      offset: { x: number; y: number }
      velocity: { x: number; y: number }
    }
  ) => {
    if (
      side === "bottom" &&
      (velocity.y > 150 || offset.y > screen.height * 0.25)
    ) {
      closeDrawer()
    }
    if (
      side === "top" &&
      (velocity.y < -150 || offset.y < screen.height * 0.25)
    ) {
      state.close()
    }
    if (side === "left" && velocity.x < -150) {
      closeDrawer()
    }
    if (side === "right" && velocity.x > 150) {
      state.close()
    }
  }

  // Velocity-weighted nearest-snap resolution. Projects the release point
  // forward by its velocity before picking the nearest snap (so a fast
  // drag that hasn't visually reached the next point yet still commits to
  // it), then separately checks a pure position-only read: if that agrees
  // with the projected pick (i.e. "stayed put") but the release was a fast
  // flick, forces one step in the fling's direction anyway -- otherwise a
  // quick short flick near a boundary would resolve to doing nothing,
  // which reads as broken/unresponsive.
  const handleMultiSnapDragEnd = (
    _: React.MouseEvent<HTMLDivElement>,
    {
      velocity,
    }: {
      offset: { x: number; y: number }
      velocity: { x: number; y: number }
    }
  ) => {
    if (!snapPointsPx || activeSnapIndex === undefined || !onSnapChange) {
      return
    }

    const current = ySpring.get()
    const projected = current + velocity.y * VELOCITY_PROJECTION_SECONDS
    const clamped = Math.min(Math.max(projected, 0), snapYs[0])

    let nearestToProjected = 0
    let nearestToProjectedDist = Infinity
    let nearestToCurrent = 0
    let nearestToCurrentDist = Infinity
    snapYs.forEach((y, i) => {
      const projectedDist = Math.abs(y - clamped)
      if (projectedDist < nearestToProjectedDist) {
        nearestToProjectedDist = projectedDist
        nearestToProjected = i
      }
      const currentDist = Math.abs(y - current)
      if (currentDist < nearestToCurrentDist) {
        nearestToCurrentDist = currentDist
        nearestToCurrent = i
      }
    })

    let resolvedIndex = nearestToProjected
    if (
      nearestToProjected === nearestToCurrent &&
      Math.abs(velocity.y) > FLING_VELOCITY_THRESHOLD
    ) {
      // y decreases toward "full" -- a fast upward drag (negative
      // velocity.y) advances the index; downward retreats it.
      const direction = velocity.y < 0 ? 1 : -1
      resolvedIndex = Math.min(
        Math.max(nearestToCurrent + direction, 0),
        snapYs.length - 1
      )
    }

    // Settle immediately regardless of whether the index actually
    // changed -- a released drag rarely lands exactly on the target
    // pixel, so this is what guarantees the sheet always ends up exactly
    // at a real snap position rather than wherever the gesture let go.
    animate(
      ySpring,
      snapYs[resolvedIndex],
      shouldReduceMotion ? { duration: 0 } : SNAP_SPRING
    )
    onSnapChange(resolvedIndex)
  }

  return (
    <AnimatePresence>
      {(props?.isOpen || state?.isOpen) && (
        <DrawerRoot
          className={twMerge(
            "bg-background text-foreground h-full max-h-full touch-none overflow-hidden align-middle ring ring-input will-change-transform",
            side === "top" &&
              (isFloat
                ? "inset-x-2 top-2 rounded-lg"
                : "inset-x-0 top-0 rounded-b-2xl"),
            side === "right" &&
              [
                "max-w-xs overflow-y-auto",
                "**:[[slot=header]]:text-start",
                isFloat
                  ? "inset-y-2 right-2 rounded-lg"
                  : "inset-y-0 right-0 h-auto",
              ].join(" "),
            side === "bottom" &&
              (isFloat
                ? "inset-x-2 bottom-2 rounded-lg"
                : "inset-x-0 bottom-0 rounded-t-2xl"),
            side === "left" &&
              [
                "overflow-y-auto",
                "**:[[slot=header]]:text-start",
                isFloat
                  ? "inset-y-2 left-2 rounded-lg"
                  : "inset-y-0 left-0 h-auto",
              ].join(" "),
            className
          )}
          style={isMultiSnapBottom ? { y: ySpring } : undefined}
          animate={
            isMultiSnapBottom ? { x: 0, opacity: 1 } : { x: 0, y: 0, opacity: 1 }
          }
          initial={
            shouldReduceMotion
              ? { opacity: 0 }
              : isMultiSnapBottom
                ? { x: 0, opacity: 1 }
                : { ...offscreen, opacity: 1 }
          }
          exit={
            shouldReduceMotion
              ? { opacity: 0 }
              : isMultiSnapBottom
                ? { x: 0, opacity: 1 }
                : { ...offscreen, opacity: 1 }
          }
          drag={side === "left" || side === "right" ? "x" : "y"}
          whileDrag={{ cursor: "grabbing" }}
          dragConstraints={
            isMultiSnapBottom
              ? { top: 0, bottom: snapYs[0], left: 0, right: 0 }
              : { top: 0, bottom: 0, left: 0, right: 0 }
          }
          dragControls={dragControls}
          dragTransition={{
            bounceStiffness: 600,
            bounceDamping: 20,
          }}
          dragMomentum={!isMultiSnapBottom}
          transition={{ duration: 0.15, ease: "easeInOut" }}
          onDragEnd={isMultiSnapBottom ? handleMultiSnapDragEnd : handleBinaryDragEnd}
          dragElastic={
            isMultiSnapBottom
              ? { top: 0.2, bottom: 0.2, left: 0, right: 0 }
              : {
                  top: side === "top" ? 1 : 0,
                  bottom: side === "bottom" ? 1 : 0,
                  left: side === "left" ? 1 : 0,
                  right: side === "right" ? 1 : 0,
                }
          }
          dragListener={false}
        >
          <Dialog
            aria-label="Drawer"
            role="dialog"
            className={twMerge(
              "relative flex h-full flex-col overflow-hidden outline-hidden will-change-auto",
              side === "top" || side === "bottom"
                ? "mx-auto max-w-lg"
                : "h-full"
            )}
          >
            <div
              className={twMerge(
                "relative bottom-0 left-0 z-15 flex h-full min-h-0 flex-col",
                // The sheet's own background still reaches the true bottom
                // edge (no change there) — this keeps its *content* clear
                // of the home indicator / gesture bar on notched phones.
                // (Set here, not on the <Dialog> above: DrawerRoot clones
                // Dialog with its own className via cloneElement, which
                // replaces rather than merges, so anything added there
                // never reaches the DOM.)
                side === "bottom" && "pb-[env(safe-area-inset-bottom)]"
              )}
            >
              {notch && side === "bottom" && (
                <div
                  className="notch absolute top-0 left-[50%] z-15 mx-auto my-2 my-2.5 h-1.5 w-10 shrink-0 translate-x-[-50%] touch-pan-y rounded-full bg-gray-400"
                  onPointerDown={(e) => dragControls.start(e)}
                  style={{ touchAction: "none" }}
                />
              )}
              {children as React.ReactNode}
              {notch && side === "left" && (
                <div
                  className="notch absolute top-[50%] right-0 z-15 mx-2.5 mx-auto h-10 w-1.5 shrink-0 translate-y-[-50%] touch-pan-y rounded-full bg-gray-400"
                  onPointerDown={(e) => dragControls.start(e)}
                  style={{ touchAction: "none" }}
                />
              )}
              {notch && side === "top" && (
                <div className="notch sticky bottom-0 z-15 mx-auto mb-2.5 h-1.5 w-10 shrink-0 touch-pan-y rounded-full bg-gray-400" />
              )}
            </div>
          </Dialog>
        </DrawerRoot>
      )}
    </AnimatePresence>
  )
}

const DrawerHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => {
  return (
    <div
      slot="header"
      className={twMerge(
        "flex flex-col p-4 text-center sm:text-start",
        className
      )}
      {...props}
    />
  )
}

const DrawerTitle = ({ className, ...props }: HeadingProps) => (
  <Heading
    slot="title"
    className={twMerge("text-lg/8 font-semibold", className)}
    {...props}
  />
)

const DrawerDescription = ({ className, ...props }: TextProps) => (
  <Text
    slot="description"
    className={twMerge("text-muted-fg text-sm", className)}
    {...props}
  />
)

const DrawerBody = ({
  className,
  ref,
  ...props
}: React.HTMLAttributes<HTMLDivElement> & { ref?: Ref<HTMLDivElement> }) => (
  <div
    ref={ref}
    slot="body"
    className={twMerge(
      "isolate flex h-full min-h-0 flex-col overflow-auto px-4 py-1 will-change-scroll",
      className
    )}
    {...props}
  />
)

const DrawerFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => {
  return (
    <div
      slot="footer"
      className={twMerge(
        "isolate mt-auto flex flex-col-reverse justify-end gap-2 p-4 sm:flex-row",
        className
      )}
      {...props}
    />
  )
}

const DrawerClose = ({ className, ...props }: ButtonProps) => {
  return <Button slot="close" className={className} {...props} />
}

const DrawerTrigger = ButtonPrimitive

export {
  Drawer,
  DrawerTrigger,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
  DrawerDescription,
  DrawerBody,
  DrawerContent,
  DrawerClose,
}
export type { DrawerContentProps }
