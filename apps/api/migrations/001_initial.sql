create extension if not exists pgcrypto;

create table app_user (
    id uuid primary key default gen_random_uuid(),
    created_at timestamptz not null default now()
);

create table user_session (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references app_user(id) on delete cascade,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    revoked_at timestamptz
);

create index user_session_user_id_idx on user_session(user_id);
create index user_session_expires_at_idx on user_session(expires_at);

create table spotify_account (
    user_id uuid primary key references app_user(id) on delete cascade,
    access_token text not null,
    refresh_token text not null,
    token_type text not null default 'Bearer',
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index spotify_account_expires_at_idx on spotify_account(expires_at);