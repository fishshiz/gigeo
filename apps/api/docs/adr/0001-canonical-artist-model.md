---
status: accepted
---

# Canonical artist identity via a persisted `artists` table

Event enrichment (Ticketmaster, PredictHQ, Apple Music, Spotify) currently decides "is this the same artist" by exact-normalized-name string comparison at read time, backed only by an in-memory cache that doesn't survive a restart and has no way to link one artist's identity across providers or events. We're introducing a persisted `artists` table — one canonical row per artist, storing `apple_music_id`, `spotify_id`, and `ticketmaster_attraction_id`, keyed for lookup by normalized name — so identity is resolved once when an artist is first enriched rather than re-derived by string comparison on every read.

Note: this system has no persisted `events` or `performers` table — events are fetched live per request from Ticketmaster/PredictHQ and streamed straight to the client, never written to Postgres. The `artists` table therefore stands alone; it's referenced by lookup (normalized name / provider ID) from the live request path, not by foreign key from an events table. This was discovered after the original design assumed events were stored, and forced two follow-on corrections — see Consequences.

## Considered Options

- **Keep normalized-name-keyed identity, just persist the existing cache to a DB table instead of in-memory.** Rejected: string comparison at read time is slower than a FK join, and has real edge cases a provider-ID-keyed row sidesteps — two distinct artists that normalize to the same string, or the same artist spelled differently across sources. It also doesn't give the events table anything stable to join against.

## Consequences

- Provider priority (Apple Music first with a verified match, Spotify as fallback) writes into a canonical row rather than a name-keyed cache entry.
- **Upcoming events reverts to a live Ticketmaster attraction-ID lookup only** — the original plan to query our own events DB for other events sharing a canonical artist doesn't work, since no events are ever persisted. PredictHQ-only artists with no Ticketmaster attraction ID simply show no upcoming events; this coverage gap is accepted rather than solved by this table.
- Enrichment isn't a batch job over stored events (there's nothing to batch over). Instead: when a performer name streams through a live Ticketmaster/PredictHQ request and has no canonical artist row (or a stale one), a bounded-concurrency background task (`tokio::spawn`, same cap style as the existing PredictHQ artwork backfill) enriches it and writes to the `artists` table without blocking that request's response. The user who triggers the first sighting of a new artist won't see it enriched yet (graceful empty state); every subsequent request for that artist will.
- Once an artist's identity is resolved (matched to a provider ID), it's treated as effectively permanent and isn't re-matched. Other fields on the row (related artists, social links, genres) go stale on their own schedule and are refreshed on a rolling TTL instead.

Full pipeline design tracked in [fishshiz/gigeo#56](https://github.com/fishshiz/gigeo/issues/56).
