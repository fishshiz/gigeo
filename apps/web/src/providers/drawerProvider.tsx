/* eslint-disable react-refresh/only-export-components */
import { useState, useContext, createContext } from "react"

type DrawerProviderState = {
  isDrawerOpen: boolean
  setIsDrawerOpen: (isOpen: boolean) => void
}

type DrawerProviderProps = {
  children: React.ReactNode
}

export const DrawerProviderContext = createContext<
  DrawerProviderState | undefined
>(undefined)

export function DrawerProvider({ children }: DrawerProviderProps) {
  const [isDrawerOpen, setIsDrawerOpen] = useState<boolean>(true)

  return (
    <DrawerProviderContext
      value={{
        isDrawerOpen,
        setIsDrawerOpen,
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
