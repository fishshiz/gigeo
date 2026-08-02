-- Caches each account's exact-normalized Spotify top-artist names, used to
-- match against Ticketmaster attractions for personalized discovery.
-- Spotify's top-artists list reflects listening history that changes slowly,
-- so it's refreshed on a TTL rather than re-fetched on every
-- /concerts/stream request (which fires on every map pan/zoom).
create table spotify_top_artist_cache (
    account_id uuid primary key references account(id) on delete cascade,
    normalized_names text[] not null,
    fetched_at timestamptz not null default now()
);
