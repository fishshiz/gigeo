type Image = {
  ratio: string
  url: string
  width: number
  height: number
  fallback: boolean
}
type ResponsiveImageProps = {
  sources: Image[]
  alt: string
  // optional: CSS width (e.g. "100%", "300px")
  style?: React.CSSProperties
}

export function ResponsiveImage({ sources, alt, style }: ResponsiveImageProps) {
  if (!sources.length) return null

  // sort by width, just to be safe
  const sorted = [...sources].sort((a, b) => a.width - b.width)

  const largest = sorted[sorted.length - 1]
  const aspectRatio = largest.width / largest.height

  const srcSet = sorted.map((s) => `${s.url} ${s.width}w`).join(", ")

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
        srcSet={srcSet}
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
