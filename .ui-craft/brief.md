# Design Brief

## 1. Product purpose (1 sentence)

Answers "what's happening near me today that I'd enjoy?" — plots nearby concerts on an interactive map, personalized to the artists the user already listens to, with a one-tap path to tickets or a seeded Spotify/Apple Music playlist.

## 2. Primary user (1 sentence)

Someone with free time today or this week, deciding what to do, browsing on their phone — asking "what's on tonight/this weekend nearby" rather than planning months in advance.

## 3. Three to five principles (the operating beliefs)

Listed in conflict-resolution order. When two principles apply to the same decision, the higher one wins.

1. **The map never searches without being asked.** Panning previews a new area; it never fires a new search until the user confirms via "Search this area." (Already shipped in #58–#61.)
2. **Speed of the first action.** No sign-in gate to browse the map. Connecting Spotify/Apple Music is optional and deferred until the user actually wants a playlist.
3. **A match to your taste is loud, not quiet.** When an event matches the user's connected music library, it's visually distinct — not a subtle badge buried in metadata.
4. **Relevance over completeness.** The app doesn't try to be an exhaustive listing. Results rank by near + soon + matches-your-taste; it's fine to under-show rather than overwhelm.

## 4. Success metric for the surface (1-2 sentences)

The user identifies a show happening near them today/soon that fits their taste, and taps through to tickets (or the artist ends up seeded into a playlist) within a single map session — without needing to sign in first.

## 5. Out of scope

- Does not sell tickets directly — always links out to Ticketmaster/the source
- Does not support venue or event submissions
- Does not have social features (no following, sharing, friend activity)
- Does not guarantee coverage beyond Ticketmaster + PredictHQ's catalogs

## 6. Learned constraints (append-only)

_None yet — this section grows as design corrections are made on this project._
