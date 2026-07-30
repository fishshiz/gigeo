# ADR-0001: Spotify / Apple Music client duplication

**Status:** Proposed
**Date:** 2026-07-29
**Deciders:** gigeo maintainers

## Context

`apps/api/src/spotify/client.rs` (487 lines) and
`apps/api/src/apple_music/client.rs` (396 lines) are independent HTTP
clients for the two music providers gigeo integrates with. A tech-debt
audit (see PRs #19–#23) flagged them as "duplicated provider clients: no
shared trait, drifting method signatures" and asked whether they should be
unified behind a common abstraction.

Both clients are live and in active use — each is instantiated in
`state.rs`, wired in `lib.rs`, and backs real routes (artist lookup,
playlist creation) in `spotify/spotify_handlers.rs` and
`apple_music/handlers.rs` respectively.

Surface-level, they look similar: both hold a `reqwest::Client`, expose
`search_*` / `get_artist` / `create_*_playlist` style methods, and have a
private `get_json` / `post_*` helper plus a `map_error` function. That
similarity is what makes them look like an obvious dedup target. Looking
closer, the two APIs diverge in every dimension that would actually matter
to a shared abstraction:

- **Auth shape.** Spotify takes one bearer token per call. Apple Music
  takes a developer token *and* an optional user token sent as a custom
  `Music-User-Token` header — a fundamentally different signature, not a
  cosmetic difference.
- **Response shape.** Spotify's JSON is flat (`Artist { id, name, uri,
  ... }`). Apple Music wraps everything in a JSON:API-style envelope
  (`DataResponse<T> { data: Vec<T> }`, resources with `id` / `type` /
  `attributes`). A shared `Artist`/`Track` domain type would need to erase
  real structural differences (e.g. Apple Music artists have no `uri`;
  Spotify tracks have no `type` discriminator).
- **Error body shape.** Spotify errors are `{ error: { status, message } }`.
  Apple Music errors are `{ errors: [{ status, title, detail }] }` (a
  list, first-one-wins). Each `map_error` already encodes this correctly
  and independently.
- **Pagination.** Spotify pages via an opaque `next` URL on `Paging<T>`.
  Apple Music's `SearchResultGroup<T>` / `ViewResponse<T>` follow the same
  `next: Option<String>` shape today, but only because both happen to
  match JSON:API-ish conventions loosely — not because of a shared
  contract either provider guarantees going forward.

Critically, the actually error-prone, worth-getting-right logic —
exponential back-off on HTTP 429, respecting `Retry-After`, retry-count
limits — is **already** shared, in `apps/api/src/http_utils.rs`
(`request_with_backoff`). Each client supplies its own `map_error` closure
so the shared function stays agnostic to error-body shape. This is the
dedup that mattered, and it's done; the audit finding is describing what's
left over after that.

There are also only two providers. A trait designed for "N future
providers" right now is a trait designed against one real data point,
since Apple Music is the first and only providers it would be generalizing
from Spotify.

## Decision

**Keep `SpotifyClient` and `AppleMusicClient` as independent
implementations. Do not introduce a shared provider trait or a unified
domain model.**

As a small, low-risk follow-up (not blocking this decision), the thin
`get_json` / `post_with_backoff` wrapper boilerplate in each client (~15
lines apiece — build a request, call `request_with_backoff`, deserialize)
could be pulled into a small generic helper in `http_utils.rs` that takes
the auth-header-setting closure as a parameter. This removes the one bit
of *incidental* duplication (calling the shared retry function) without
touching domain types or auth signatures. This is optional cleanup, sized
at under an hour, and not required by this ADR.

## Options Considered

### Option A: Keep independent implementations (chosen)

| Dimension | Assessment |
|---|---|
| Complexity | None added — status quo |
| Cost | None |
| Scalability (to a 3rd provider) | New client copies the pattern; ~5 min of extra typing, no framework to learn |
| Team familiarity | Already familiar — no new abstraction to onboard onto |

**Pros:**
- No abstraction tax: reading `spotify/client.rs` tells you everything about Spotify, with no indirection through a trait or generic domain type.
- The genuinely reusable, hard-to-get-right part (429 back-off) is already shared.
- Adding a 3rd provider later is a known, low-cost pattern to copy — and by then there'd be real data (3 shapes, not 2) to design a trait from, if one still makes sense.

**Cons:**
- Method-name/shape drift between the two clients (e.g. `search_artist` vs `search_artists`) isn't structurally prevented — only caught by code review.
- A future provider (a 3rd option) still starts from copy-paste.

### Option B: Full shared trait + unified domain model

| Dimension | Assessment |
|---|---|
| Complexity | High — needs a domain model that abstracts over materially different auth (1 token vs. 2) and response shapes (flat vs. JSON:API envelope) |
| Cost | Non-trivial refactor of both clients and their ~15 call sites across handlers/services |
| Scalability | Better *if* a 3rd provider arrives and fits the same trait shape; worse if it doesn't (Apple Music already barely fits with Spotify) |
| Team familiarity | New generic/trait-object machinery to learn and maintain |

**Pros:**
- A single `trait MusicProvider` would make provider-agnostic code (e.g. "search an artist, on whichever provider the user connected") possible without an if/else on provider type.
- Structurally prevents method-signature drift.

**Cons:**
- The two providers' auth and response shapes are different enough that the trait's methods would need lowest-common-denominator signatures (e.g. `Option<&str>` for a "second token" nobody but Apple Music uses) or an associated-type escape hatch per provider — both are classic signs the abstraction doesn't fit the domain yet.
- Nothing in the codebase currently *needs* provider-agnostic dispatch — every call site already knows which provider it's talking to (it's calling `state.spotify.*` or `state.apple_music_client.*` directly, gated by which account the user connected).
- Two data points is not enough to safely generalize a trait's shape from; it's likely to need reshaping again when (if) a 3rd provider shows up, at which point the trait becomes an obstacle rather than a guide.

### Option C: Extract only the thin request-wrapper boilerplate (adopted as optional follow-up)

| Dimension | Assessment |
|---|---|
| Complexity | Low — a few lines of generic glue in `http_utils.rs` |
| Cost | Under an hour |
| Scalability | Neutral — doesn't constrain how a 3rd client's domain model looks |
| Team familiarity | Trivial, no new concepts |

**Pros:** removes the one piece of duplication that's purely mechanical (the `get_json`/`post_json` shape), with no cost to readability.
**Cons:** doesn't address method-name drift or give provider-agnostic dispatch — but nothing currently needs those.

## Trade-off Analysis

The real question a shared trait answers is "can calling code treat
Spotify and Apple Music interchangeably?" Nothing in gigeo needs that
today — every call site is already provider-specific because the user
explicitly connects one account type or the other, and the two providers'
playlist-creation flows differ enough (Apple Music needs a separate
`Music-User-Token`; Spotify doesn't) that a caller couldn't meaningfully
be agnostic to which one it's calling anyway.

What looked like "duplicated business logic" in the audit is mostly
duplicated *plumbing* around genuinely different domain models — and the
one piece of plumbing that was worth sharing (retry/back-off) already is.
Forcing a trait over the remaining differences would trade real
duplication (which is visible, local, and cheap to read) for an
abstraction that has to paper over a token-count mismatch and two
unrelated JSON shapes — a worse trade.

## Consequences

- Adding a 3rd music provider still means writing a new client from
  scratch, following the existing two as a template. That's an accepted
  cost, not a gap to fix now.
- Method-name drift between clients (already present, e.g.
  `search_artist` vs `search_artists`) is not structurally prevented.
  Code review is the guard here.
- If a 3rd provider is added later with a response/auth shape that
  actually matches Spotify or Apple Music's pattern, revisit this ADR —
  three real implementations is enough data to design a trait from, two
  isn't.

## Action Items

1. [ ] (Optional, unblocked, low priority) Extract the `get_json` /
       `post_*` wrapper boilerplate into a small generic helper in
       `http_utils.rs`, parameterized by an auth-header-setting closure.
       Est. <1 hour. Not required to close this ADR.
2. [ ] No action required on the trait/domain-model question — revisit
       only if a 3rd provider is added.
