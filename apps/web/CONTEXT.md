# Web

React/Vite frontend for Gigeo — the map/search UI, event drawer, and Spotify/Apple Music playlist integrations.

## Language

**LocalDay**:
The calendar day an event or search boundary falls on in the viewer's own timezone. Represented as a `YYYY-MM-DD` string. Owned by `lib/dates.ts` (`eventDateKey`, `dateRangeToApiParams`). Assumes the viewer's timezone approximates the venue's — the app has no reliable source for the venue's actual timezone.
_Avoid_: CalendarDay (the backend's `reconcile.rs` has a `calendar_day` function that means the venue-*local* day, sourced from each provider's own local-time field (Ticketmaster's `localDate`, PredictHQ's `start_local`) — a different concept, used only for cross-source event matching, that happens to share a name with LocalDay above. Still don't conflate the two: CalendarDay is backend-only and never sent to the frontend), UTC day.
