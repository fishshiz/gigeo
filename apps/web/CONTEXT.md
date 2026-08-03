# Web

React/Vite frontend for Gigeo — the map/search UI, event drawer, and Spotify/Apple Music playlist integrations.

## Language

**LocalDay**:
The calendar day an event or search boundary falls on in the viewer's own timezone. Represented as a `YYYY-MM-DD` string. Owned by `lib/dates.ts` (`eventDateKey`, `dateRangeToApiParams`). Assumes the viewer's timezone approximates the venue's — the app has no reliable source for the venue's actual timezone.
_Avoid_: CalendarDay (the backend's `reconcile.rs` has a `calendar_day` function that means the *UTC* day, a different concept used for cross-source event matching — don't conflate the two), UTC day.
