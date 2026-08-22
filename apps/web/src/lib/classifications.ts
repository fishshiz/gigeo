// Ticketmaster labels some classifications as "Undefined" -- fold those
// into "Miscellaneous" (another TM classification) so both surface as one
// filter option instead of two near-duplicates.
const CLASSIFICATION_ALIASES: Record<string, string> = {
  Undefined: "Miscellaneous",
}

export function normalizeClassificationName(name: string): string {
  return CLASSIFICATION_ALIASES[name] ?? name
}
