create table spotify_playlist (
    id uuid primary key default gen_random_uuid(),
    spotify_account_user_id text not null references spotify_account(spotify_user_id) on delete cascade,
    spotify_playlist_id text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);