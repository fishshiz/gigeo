create extension if not exists pgcrypto;

-- ============================================================
-- ENUMS
-- ============================================================

create type music_provider as enum ('spotify', 'apple_music');
create type playlist_visibility as enum ('public', 'private');
create type playlist_update_mode as enum ('additive', 'destructive');
create type playlist_update_run_status as enum ('pending', 'running', 'success', 'failed');

-- ============================================================
-- ACCOUNTS (class-table-inheritance: account -> {spotify,apple_music}_account)
-- ============================================================

create table account (
    id uuid primary key default gen_random_uuid(),
    provider music_provider not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table spotify_account (
    account_id uuid primary key references account(id) on delete cascade,
    spotify_user_id text not null unique,
    scope text not null,
    access_token text not null,
    refresh_token text not null,
    token_type text not null default 'Bearer',
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index spotify_account_expires_at_idx on spotify_account(expires_at);

create table apple_music_account (
    account_id uuid primary key references account(id) on delete cascade,
    apple_music_user_id text not null unique,   -- your own mapping, Apple doesn't give a stable user id via MusicKit
    music_user_token text not null,             -- MusicKit user token
    storefront text not null,                   -- e.g. 'us'
    token_created_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

-- user_session now points at the generic account, so it works for either provider
create table user_session (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references account(id) on delete cascade,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    revoked_at timestamptz
);

create index user_session_account_id_idx on user_session(account_id);
create index user_session_expires_at_idx on user_session(expires_at);

-- ============================================================
-- PLAYLISTS
-- ============================================================

create table playlist (
    id uuid primary key default gen_random_uuid(),
    account_id uuid not null references account(id) on delete cascade,
    provider_playlist_id text not null,
    name text not null,

    geohash text not null check (char_length(geohash) = 6),
    city text not null,

    visibility playlist_visibility not null default 'private',
    update_mode playlist_update_mode not null default 'additive',
    update_cadence_days smallint not null check (update_cadence_days in (7, 30, 60)),

    is_active boolean not null default true,
    last_updated_at timestamptz,
    next_update_at timestamptz,

    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),

    unique (account_id, provider_playlist_id)
);

create index playlist_geohash_idx on playlist (geohash);
create index playlist_scheduler_idx on playlist (next_update_at) where is_active;
create index playlist_account_id_idx on playlist (account_id);

-- ============================================================
-- GENRE PRIORITIZATION (per playlist)
-- ============================================================

create table playlist_genre_priority (
    playlist_id uuid not null references playlist(id) on delete cascade,
    genre text not null,
    priority smallint not null check (priority > 0),  -- lower = higher priority
    created_at timestamptz not null default now(),
    primary key (playlist_id, genre),
    unique (playlist_id, priority)
);

-- ============================================================
-- UPDATE RUN HISTORY (audit trail for each update job)
-- ============================================================

create table playlist_update_run (
    id uuid primary key default gen_random_uuid(),
    playlist_id uuid not null references playlist(id) on delete cascade,
    status playlist_update_run_status not null default 'pending',
    update_mode playlist_update_mode not null,     -- snapshot in case config changes later
    started_at timestamptz not null default now(),
    finished_at timestamptz,
    tracks_added integer not null default 0,
    tracks_removed integer not null default 0,
    artists_added jsonb not null default '[]'::jsonb,
    error_message text
);

create index playlist_update_run_playlist_id_idx on playlist_update_run (playlist_id, started_at desc);

-- ============================================================
-- updated_at trigger (reusable)
-- ============================================================

create or replace function set_updated_at()
returns trigger as $$
begin
    new.updated_at = now();
    return new;
end;
$$ language plpgsql;

create trigger account_set_updated_at before update on account
    for each row execute function set_updated_at();
create trigger spotify_account_set_updated_at before update on spotify_account
    for each row execute function set_updated_at();
create trigger apple_music_account_set_updated_at before update on apple_music_account
    for each row execute function set_updated_at();
create trigger playlist_set_updated_at before update on playlist
    for each row execute function set_updated_at();