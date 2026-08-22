import { describe, expect, it } from "vitest"
import { parseCreatePlaylistForm, parseUpdatePlaylistForm } from "./spotify"

function makeForm(fields: Record<string, string>): FormData {
  const form = new FormData()
  for (const [key, value] of Object.entries(fields)) {
    form.set(key, value)
  }
  return form
}

const validFields = {
  playlistName: "Road Trip",
  location: "Austin, TX",
  cadence: "weekly",
}

describe("parseCreatePlaylistForm", () => {
  it("parses a valid form into a CreatePlaylistInput", () => {
    expect(parseCreatePlaylistForm(makeForm(validFields))).toEqual({
      name: "Road Trip",
      location: "Austin, TX",
      description: "",
      privacy: false,
      cadence: 7,
      destructive: false,
      radius: 25,
      genres: [],
    })
  })

  it("trims whitespace from name and location", () => {
    const result = parseCreatePlaylistForm(
      makeForm({ ...validFields, playlistName: "  Road Trip  ", location: "  Austin, TX  " })
    )
    expect(result.name).toBe("Road Trip")
    expect(result.location).toBe("Austin, TX")
  })

  it("throws when name is missing", () => {
    const form = makeForm({ ...validFields })
    form.delete("playlistName")
    expect(() => parseCreatePlaylistForm(form)).toThrow(
      "Playlist name is required"
    )
  })

  it("throws when name is blank", () => {
    expect(() =>
      parseCreatePlaylistForm(makeForm({ ...validFields, playlistName: "   " }))
    ).toThrow("Playlist name is required")
  })

  it("throws when location is missing", () => {
    const form = makeForm({ ...validFields })
    form.delete("location")
    expect(() => parseCreatePlaylistForm(form)).toThrow(
      "Location is required"
    )
  })

  it("throws when cadence is missing", () => {
    const form = makeForm({ ...validFields })
    form.delete("cadence")
    expect(() => parseCreatePlaylistForm(form)).toThrow(
      "Cadence is required"
    )
  })

  it.each([
    ["weekly", 7],
    ["bimonthly", 60],
    ["monthly", 30],
    ["anything-else", 30],
  ])("maps cadence %s to %i days", (cadence, expected) => {
    const result = parseCreatePlaylistForm(
      makeForm({ ...validFields, cadence })
    )
    expect(result.cadence).toBe(expected)
  })

  it("sets privacy true only when privacy field is 'private'", () => {
    expect(
      parseCreatePlaylistForm(makeForm({ ...validFields, privacy: "private" }))
        .privacy
    ).toBe(true)
    expect(
      parseCreatePlaylistForm(makeForm({ ...validFields, privacy: "public" }))
        .privacy
    ).toBe(false)
  })

  it("sets destructive true only when behavior field is 'destructive'", () => {
    expect(
      parseCreatePlaylistForm(
        makeForm({ ...validFields, behavior: "destructive" })
      ).destructive
    ).toBe(true)
    expect(
      parseCreatePlaylistForm(
        makeForm({ ...validFields, behavior: "preserve" })
      ).destructive
    ).toBe(false)
  })

  it("never sends a hardcoded placeholder description", () => {
    expect(parseCreatePlaylistForm(makeForm(validFields)).description).toBe(
      ""
    )
  })

  it("parses the genres hidden field from JSON", () => {
    const result = parseCreatePlaylistForm(
      makeForm({ ...validFields, genres: JSON.stringify(["Rock", "Pop"]) })
    )
    expect(result.genres).toEqual(["Rock", "Pop"])
  })

  it("defaults genres to an empty array when the field is missing", () => {
    expect(parseCreatePlaylistForm(makeForm(validFields)).genres).toEqual([])
  })

  it("defaults genres to an empty array when the field is malformed JSON", () => {
    const result = parseCreatePlaylistForm(
      makeForm({ ...validFields, genres: "not json" })
    )
    expect(result.genres).toEqual([])
  })
})

const validUpdateFields = {
  playlistName: "Road Trip",
  cadence: "weekly",
}

describe("parseUpdatePlaylistForm", () => {
  it("parses a valid form into an UpdatePlaylistInput", () => {
    expect(parseUpdatePlaylistForm(makeForm(validUpdateFields))).toEqual({
      name: "Road Trip",
      privacy: false,
      cadence: 7,
      destructive: false,
      genres: [],
    })
  })

  it("trims whitespace from name", () => {
    const result = parseUpdatePlaylistForm(
      makeForm({ ...validUpdateFields, playlistName: "  Road Trip  " })
    )
    expect(result.name).toBe("Road Trip")
  })

  it("throws when name is missing", () => {
    const form = makeForm({ ...validUpdateFields })
    form.delete("playlistName")
    expect(() => parseUpdatePlaylistForm(form)).toThrow(
      "Playlist name is required"
    )
  })

  it("throws when name is blank", () => {
    expect(() =>
      parseUpdatePlaylistForm(
        makeForm({ ...validUpdateFields, playlistName: "   " })
      )
    ).toThrow("Playlist name is required")
  })

  it("throws when cadence is missing", () => {
    const form = makeForm({ ...validUpdateFields })
    form.delete("cadence")
    expect(() => parseUpdatePlaylistForm(form)).toThrow(
      "Cadence is required"
    )
  })

  it.each([
    ["weekly", 7],
    ["bimonthly", 60],
    ["monthly", 30],
    ["anything-else", 30],
  ])("maps cadence %s to %i days", (cadence, expected) => {
    const result = parseUpdatePlaylistForm(
      makeForm({ ...validUpdateFields, cadence })
    )
    expect(result.cadence).toBe(expected)
  })

  it("sets privacy true only when privacy field is 'private'", () => {
    expect(
      parseUpdatePlaylistForm(
        makeForm({ ...validUpdateFields, privacy: "private" })
      ).privacy
    ).toBe(true)
    expect(
      parseUpdatePlaylistForm(
        makeForm({ ...validUpdateFields, privacy: "public" })
      ).privacy
    ).toBe(false)
  })

  it("sets destructive true only when behavior field is 'destructive'", () => {
    expect(
      parseUpdatePlaylistForm(
        makeForm({ ...validUpdateFields, behavior: "destructive" })
      ).destructive
    ).toBe(true)
    expect(
      parseUpdatePlaylistForm(
        makeForm({ ...validUpdateFields, behavior: "preserve" })
      ).destructive
    ).toBe(false)
  })
})
