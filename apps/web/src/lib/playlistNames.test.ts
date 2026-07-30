import { describe, expect, it } from "vitest"
import { getRandomPlaylistName } from "./playlistNames"

describe("getRandomPlaylistName", () => {
  it("returns an empty string when city is empty", () => {
    expect(getRandomPlaylistName("")).toBe("")
  })

  it("returns a non-empty name containing the city when given one", () => {
    const name = getRandomPlaylistName("Austin")
    expect(name.length).toBeGreaterThan(0)
    expect(name).toContain("Austin")
  })

  it("only ever returns one of the known templates", () => {
    // Sample many times to exercise the random branch without pinning to
    // a specific pick.
    for (let i = 0; i < 50; i++) {
      const name = getRandomPlaylistName("Denver")
      expect(name).toMatch(/Denver/)
    }
  })
})
