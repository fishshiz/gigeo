type Artwork = {
  url: string
  bgColor?: string
}

export const buildArtworkUrl = (artwork: Artwork, size: number) => {
  return artwork.url.replace("{w}", String(size)).replace("{h}", String(size))
}

export const normalizeBg = (bgColor?: string) => {
  if (!bgColor) return "#111827" // fallback
  return bgColor.startsWith("#") ? bgColor : `#${bgColor}`
}
