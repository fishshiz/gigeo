import { ImageOff } from "lucide-react"

type Image = {
  ratio?: string
  url: string
  width?: number
  height?: number
  fallback?: boolean
}
type ResponsiveImageProps = {
  sources: Image[]
  alt: string
  // optional: CSS width (e.g. "100%", "300px")
  style?: React.CSSProperties
}

export function ResponsiveImage({ sources, alt, style }: ResponsiveImageProps) {
  if (!sources.length) {
    return (
      <div
        style={style}
        className="flex h-full min-h-16 w-full items-center justify-center bg-neutral-100 text-neutral-300 dark:bg-neutral-800 dark:text-neutral-600"
      >
        <ImageOff aria-hidden size={20} />
      </div>
    )
  }

  // sort by width, just to be safe
  const sorted = [...sources].sort((a, b) => (a.width ?? 0) - (b.width ?? 0))

  const largest = sorted[sorted.length - 1]
  // Ticketmaster doesn't always report dimensions; fall back to a square box
  // instead of producing a NaN aspect ratio when they're missing.
  const aspectRatio =
    largest.width && largest.height ? largest.width / largest.height : 1

  const srcSet = sorted
    .filter((s) => s.width)
    .map((s) => `${s.url} ${s.width}w`)
    .join(", ")

  // Example sizes: full width on mobile, 50vw ≥768px
  const sizes = "(max-width: 768px) 100vw, 50vw"

  return (
    <div
      style={{
        position: "relative",
        width: "100%",
        ...style,
        // reserve height based on aspect ratio to avoid layout shift
        paddingBottom: `${100 / aspectRatio}%`,
        overflow: "hidden",
      }}
    >
      <img
        src={largest.url} // fallback
        srcSet={srcSet || undefined}
        sizes={sizes}
        alt={alt}
        loading="lazy"
        style={{
          position: "absolute",
          inset: 0,
          width: "100%",
          height: "100%",
          objectFit: "cover", // or "contain"
        }}
      />
    </div>
  )
}
