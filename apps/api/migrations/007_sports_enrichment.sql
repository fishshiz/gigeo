-- Sports standings/record enrichment for Sports-segment events, gated to
-- the major leagues (see `sports_league`). Same no-persisted-events
-- constraint as `artists` (migrations/006_artists.sql, ADR-0001) applies
-- here too: there's no `events` or `performers` table, so both tables
-- below are looked up from the live Ticketmaster request path by
-- ticketmaster_attraction_id / (league, espn_team_id), never joined from
-- a stored event.
--
-- Two tables, not one, because the two caches go stale on different
-- timescales: a team's identity (which ESPN team a Ticketmaster
-- attraction name resolves to) is effectively permanent once resolved,
-- the same way `artists.matched_via` is treated as permanent per
-- ADR-0001. A team's current record/standing is not -- it changes with
-- every game -- so it gets its own row with a `fetched_at` the read path
-- can check a TTL against, independent of the (unchanging) match.
--
-- Data source is ESPN's public (undocumented) standings API, not a paid
-- provider -- chosen after API-SPORTS' free tier turned out to gate
-- current-season data behind a paid plan (confirmed live during this
-- feature's design), which isn't worth it for a portfolio project. ESPN
-- needs no API key and its one `/standings` call per league already
-- returns every team's record, seed, and conference in one shot, so
-- there's no separate "team id" from a numeric per-provider id space to
-- store -- `espn_team_id` is just ESPN's own team id (a small stable
-- string, e.g. "8").

create type sports_league as enum ('nba', 'nfl', 'nhl', 'mlb');

-- One row per Ticketmaster attraction ever seen on an eligible (Sports,
-- major-league, >=2 performers) event. `espn_team_id` null means
-- "searched, no confident match" -- a tombstone, same purpose as
-- `artists`' unmatched rows, so a name that will never resolve (a typo,
-- a non-team attraction Ticketmaster still tagged with a major-league
-- subGenre) isn't re-searched on every request that includes it.
create table sports_team_matches (
    ticketmaster_attraction_id text primary key,
    league sports_league not null,
    espn_team_id text,
    resolved_name text,

    last_attempted_at timestamptz not null default now(),
    created_at timestamptz not null default now()
);

-- Enrichment worker's retry gate for unmatched rows, mirroring
-- `artists_last_attempted_at_idx`.
create index sports_team_matches_unmatched_idx on sports_team_matches (last_attempted_at) where espn_team_id is null;

-- Current record/standing for a resolved team. Upserted wholesale on
-- every refresh (no history kept) -- nothing today reads a past
-- standing, only "as of now". `record` is ESPN's own formatted summary
-- (e.g. "60-22", or "53-22-7" for a hockey team's wins-losses-OT-losses)
-- rather than separate wins/losses columns -- the shape of "a record"
-- genuinely varies by sport, and ESPN already computes the right string
-- per sport, so there's nothing to gain by re-splitting it.
create table sports_team_standings (
    league sports_league not null,
    espn_team_id text not null,
    record text not null,
    standing_position integer,
    group_name text,

    fetched_at timestamptz not null default now(),

    primary key (league, espn_team_id)
);
