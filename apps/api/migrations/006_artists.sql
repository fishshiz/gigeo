-- Canonical artist identity, replacing the in-memory, name-keyed
-- ArtworkCache. See apps/api/docs/adr/0001-canonical-artist-model.md for
-- the reasoning, including why this table has no foreign key to events or
-- performers: neither is persisted anywhere in this system, so a row here
-- is looked up by normalized_name from the live Ticketmaster/PredictHQ
-- request path, not joined from a stored event.
--
-- Once an artist has a provider match, that identity (apple_music_id /
-- spotify_id / ticketmaster_attraction_id, matched_via, matched_at) is
-- treated as permanent and never re-resolved. The remaining enrichment
-- fields (genres, similar_artists, artwork) go stale on their own and are
-- refreshed independently via enrichment_refreshed_at.
--
-- A row can also exist unmatched (all provider ids null) as a tombstone
-- for a name that was searched and found nothing confident — mirroring
-- ArtworkCache's existing "cache misses too" behavior, so a noisy
-- PredictHQ title isn't re-searched on every request. last_attempted_at
-- gates how soon an unmatched row is retried.

create table artists (
    id uuid primary key default gen_random_uuid(),
    normalized_name text not null unique,
    display_name text not null,

    apple_music_id text,
    spotify_id text,
    ticketmaster_attraction_id text,
    matched_via music_provider,
    matched_at timestamptz,

    artwork_url text,
    artwork_bg_color text,
    apple_music_url text,
    spotify_url text,
    genres text[] not null default '{}',
    similar_artists jsonb not null default '[]'::jsonb,

    last_attempted_at timestamptz not null default now(),
    enrichment_refreshed_at timestamptz,

    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

-- Partial unique indexes: null provider ids are common (unmatched rows,
-- or matched via only one of the two providers), and null != null under a
-- plain unique constraint would otherwise still need this same shape.
create unique index artists_apple_music_id_idx on artists (apple_music_id) where apple_music_id is not null;
create unique index artists_spotify_id_idx on artists (spotify_id) where spotify_id is not null;
create unique index artists_ticketmaster_attraction_id_idx on artists (ticketmaster_attraction_id) where ticketmaster_attraction_id is not null;

-- Enrichment task's retry/refresh gate: find rows due for a first attempt
-- retry (unmatched) or a dynamic-field refresh (matched but stale).
create index artists_last_attempted_at_idx on artists (last_attempted_at);
create index artists_enrichment_refreshed_at_idx on artists (enrichment_refreshed_at) where matched_via is not null;

create trigger artists_set_updated_at before update on artists
    for each row execute function set_updated_at();
