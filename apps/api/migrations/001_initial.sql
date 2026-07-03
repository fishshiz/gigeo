create extension if not exists pgcrypto;

create table spotify_account (
    id text unique not null,
    scope text not null,
    access_token text not null,
    refresh_token text not null,
    token_type text not null default 'Bearer',
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table user_session (
    id uuid primary key default gen_random_uuid(),
    spotify_user_id text not null references spotify_account(id) on delete cascade,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    revoked_at timestamptz
);

create index user_session_spotify_user_id_idx on user_session(id);
create index user_session_expires_at_idx on user_session(expires_at);



create index spotify_account_id_idx on spotify_account(id);
create index spotify_account_expires_at_idx on spotify_account(expires_at);

create table spotify_playlist (
    id uuid primary key default gen_random_uuid(),
    spotify_account_user_id text not null references spotify_account(id) on delete cascade,
    spotify_playlist_id text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);