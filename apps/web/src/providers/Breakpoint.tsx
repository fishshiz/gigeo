// BreakpointContext.tsx
import { createContext, useContext, useEffect, useState } from "react"

const BREAKPOINTS = { sm: 640, md: 768, lg: 1024, xl: 1280 } as const
type Breakpoint = "xs" | keyof typeof BREAKPOINTS

function getBreakpoint(width: number): Breakpoint {
  if (width >= BREAKPOINTS.xl) return "xl"
  if (width >= BREAKPOINTS.lg) return "lg"
  if (width >= BREAKPOINTS.md) return "md"
  if (width >= BREAKPOINTS.sm) return "sm"
  return "xs"
}

const BreakpointContext = createContext<Breakpoint>("md")

export function BreakpointProvider({
  children,
}: {
  children: React.ReactNode
}) {
  const [bp, setBp] = useState<Breakpoint>(() =>
    typeof window !== "undefined" ? getBreakpoint(window.innerWidth) : "md"
  )

  useEffect(() => {
    const onResize = () => setBp(getBreakpoint(window.innerWidth))
    window.addEventListener("resize", onResize)
    return () => window.removeEventListener("resize", onResize)
  }, [])

  return (
    <BreakpointContext.Provider value={bp}>
      {children}
    </BreakpointContext.Provider>
  )
}

export function useBreakpoint() {
  return useContext(BreakpointContext)
}

// Derived convenience hook
const useIsMobile = () => {
  const bp = useBreakpoint()
  return bp === "xs" || bp === "sm"
}

export { useIsMobile }
