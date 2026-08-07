/** Shared motion tokens for `motion/react` `transition` props -- durations
 * are in seconds (what motion/react's API expects), not the ms-based CSS
 * scale. Centralizes the values already duplicated across EventDetails.tsx
 * instead of each new animated component re-deriving its own duration/
 * easing pair. Grown from what's actually in use, not a speculative full
 * scale -- add a step here when a real second use case needs it. */

export const MOTION_DURATION = {
  /** Small utility crossfades -- e.g. swapping loading/error/loaded
   * content in a list. */
  fast: 0.15,
  /** Small UI appearing/disappearing -- toggles, sticky headers, controls. */
  base: 0.2,
} as const

export const MOTION_EASE = {
  /** Entrances, most UI. cubic-bezier(0.22, 1, 0.36, 1). */
  out: [0.22, 1, 0.36, 1],
} as const
