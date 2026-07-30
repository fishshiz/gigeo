import { describe, expect, it } from "vitest"
import { buildArtworkUrl, normalizeBg } from "./artwork"

describe("buildArtworkUrl", () => {
  it("substitutes width and height placeholders with the given size", () => {
    const url = buildArtworkUrl(
      { url: "https://example.com/art/{w}x{h}bb.jpg" },
      300
    )
    expect(url).toBe("https://example.com/art/300x300bb.jpg")
  })

  it("leaves a url with no placeholders untouched", () => {
    const url = buildArtworkUrl({ url: "https://example.com/art.jpg" }, 300)
    expect(url).toBe("https://example.com/art.jpg")
  })
})

describe("normalizeBg", () => {
  it("falls back to a default color when bgColor is missing", () => {
    expect(normalizeBg(undefined)).toBe("#111827")
  })

  it("passes through a color that already has a # prefix", () => {
    expect(normalizeBg("#ff00aa")).toBe("#ff00aa")
  })

  it("adds a # prefix to a bare hex color", () => {
    expect(normalizeBg("ff00aa")).toBe("#ff00aa")
  })
})
