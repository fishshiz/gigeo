-- Phase 5 concert enrichment (#102): Spotify artist popularity, plus a
-- lazily-backfilled "latest release" snapshot used to flag recently
-- released music. Both are nullable, best-effort fields on the existing
-- `artists` row -- same no-persisted-events constraint as everything else
-- in this table (ADR-0001): looked up by normalized_name, never joined
-- from a stored event.
--
-- `popularity` only ever comes from Spotify (Apple Music's API doesn't
-- expose an equivalent score) -- null for an artist matched via Apple
-- Music with no concurrent Spotify match, same "one provider's data,
-- possibly missing" shape as `spotify_id`/`spotify_url` already have.
--
-- Latest-release fields are populated on demand (see
-- `artists::worker::backfill_release_info`), not at first-match time --
-- mirrors why similar_artists is backfilled lazily rather than eagerly
-- (`artists::worker::resolve_fresh`'s doc comment): most newly-seen
-- artists are never opened, so an extra `/artists/{id}/albums` call for
-- every one of them would mostly be wasted. Unlike similar_artists
-- (fetched once, rarely changes), an artist's latest release changes
-- over time, so it gets its own `release_checked_at` TTL gate
-- (`artists::worker::RELEASE_CHECK_TTL`) independent of
-- `enrichment_refreshed_at`'s 30-day genre/similar-artist cadence --
-- checking every few days keeps the "is this recent" flag from going
-- stale for long without re-fetching on every single detail-view open.
--
-- The row stores whatever the actual latest release is, even if it's
-- years old -- "is it recent" is a read-time computation (comparing
-- latest_release_date against now(), see
-- `artists::lookup::to_response`), not a write-time filter, so the
-- 30-day recency window can change later without needing a re-fetch.

alter table artists
    add column popularity smallint,
    add column latest_release_name text,
    add column latest_release_date date,
    add column latest_release_url text,
    add column latest_release_artwork_url text,
    add column release_checked_at timestamptz;
