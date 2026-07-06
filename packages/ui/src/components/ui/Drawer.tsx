"use client"

import { AnimatePresence, motion, useDragControls } from "motion/react"
import { use, Children, cloneElement } from "react"
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
import { twJoin, twMerge } from "tailwind-merge"
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
  isBlurred?: boolean
  className?: string
  side?: "top" | "bottom" | "left" | "right"
  notch?: boolean
  closeDrawer: () => void
}

const DrawerContent = ({
  side = "bottom",
  isFloat = false,
  isBlurred = true,
  notch = true,
  closeDrawer = () => {},
  children,
  className,
  ...props
}: DrawerContentProps) => {
  const state = use(OverlayTriggerStateContext)!
  const dragControls = useDragControls()
  return (
    <AnimatePresence>
      {(props?.isOpen || state?.isOpen) && (
        <DrawerRoot
          className={twJoin(
            "bg-bg text-fg h-full max-h-full touch-none overflow-hidden align-middle ring ring-input will-change-transform",
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
          animate={{ x: 0, y: 0 }}
          initial={{
            x: side === "left" ? "-100%" : side === "right" ? "100%" : 0,
            y: side === "top" ? "-100%" : side === "bottom" ? "100%" : 0,
          }}
          exit={{
            x: side === "left" ? "-100%" : side === "right" ? "100%" : 0,
            y: side === "top" ? "-100%" : side === "bottom" ? "100%" : 0,
          }}
          drag={side === "left" || side === "right" ? "x" : "y"}
          whileDrag={{ cursor: "grabbing" }}
          dragConstraints={{
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
          }}
          dragControls={dragControls}
          dragTransition={{
            bounceStiffness: 600,
            bounceDamping: 20,
          }}
          transition={{ duration: 0.15, ease: "easeInOut" }}
          onDragEnd={(
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
          }}
          dragElastic={{
            top: side === "top" ? 1 : 0,
            bottom: side === "bottom" ? 1 : 0,
            left: side === "left" ? 1 : 0,
            right: side === "right" ? 1 : 0,
          }}
          dragListener={false}
        >
          <Dialog
            aria-label="Drawer"
            role="dialog"
            className={twJoin(
              "relative flex h-full flex-col overflow-hidden outline-hidden will-change-auto",
              side === "top" || side === "bottom"
                ? "mx-auto max-w-lg"
                : "h-full"
            )}
          >
            <div className="relative bottom-0 left-0 z-15">
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
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    slot="body"
    className={twMerge(
      "isolate flex h-full max-h-[calc(var(--visual-viewport-height)-var(--visual-viewport-vertical-padding))] flex-col overflow-auto px-4 py-1 will-change-scroll",
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
