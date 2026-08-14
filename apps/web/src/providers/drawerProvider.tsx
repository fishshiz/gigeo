/* eslint-disable react-refresh/only-export-components */
import { useState, useContext, createContext } from "react"

/** Mobile-only bottom-sheet height. Desktop's sidebar has no notion of
 * this -- it only ever reads isDrawerOpen. "peek" is the practical floor:
 * there's no true "closed" state on mobile (no close button renders
 * there), so peek stands in for it. */
export type DrawerSnapPoint = "peek" | "half" | "full"

type DrawerProviderState = {
  isDrawerOpen: boolean
  setIsDrawerOpen: (isOpen: boolean) => void
  snapPoint: DrawerSnapPoint
  setSnapPoint: (point: DrawerSnapPoint) => void
}

type DrawerProviderProps = {
  children: React.ReactNode
}

export const DrawerProviderContext = createContext<
  DrawerProviderState | undefined
>(undefined)

export function DrawerProvider({ children }: DrawerProviderProps) {
  const [isDrawerOpen, setIsDrawerOpen] = useState<boolean>(true)
  const [snapPoint, setSnapPoint] = useState<DrawerSnapPoint>("peek")

  return (
    <DrawerProviderContext
      value={{
        isDrawerOpen,
        setIsDrawerOpen,
        snapPoint,
        setSnapPoint,
      }}
    >
      {children}
    </DrawerProviderContext>
  )
}

export const useDrawerProvider = () => {
  const context = useContext(DrawerProviderContext)

  if (context === undefined) {
    throw new Error("useDrawerProvider must be used within a DrawerProvider")
  }

  return context
}
