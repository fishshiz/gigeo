-- Personalization now also expands via Apple Music's similar-artists view,
-- so the cache needs to remember *why* a name matched (a direct Spotify top
-- artist vs. similar to one of them) rather than just a flat list of names.
alter table spotify_top_artist_cache drop column normalized_names;
alter table spotify_top_artist_cache add column matches jsonb not null default '{}'::jsonb;
