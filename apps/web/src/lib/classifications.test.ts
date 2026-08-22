import { describe, expect, it } from "vitest"
import { normalizeClassificationName } from "./classifications"

describe("normalizeClassificationName", () => {
  it("folds Undefined into Miscellaneous", () => {
    expect(normalizeClassificationName("Undefined")).toBe("Miscellaneous")
  })

  it("leaves other names untouched", () => {
    expect(normalizeClassificationName("Music")).toBe("Music")
    expect(normalizeClassificationName("Miscellaneous")).toBe("Miscellaneous")
  })
})
