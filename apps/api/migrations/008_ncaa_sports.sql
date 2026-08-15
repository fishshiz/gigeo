-- Extends sports enrichment (migrations/007_sports_enrichment.sql) to NCAA
-- football and basketball, alongside the four pro major leagues. Purely
-- additive -- new enum values only, no changes to existing rows/tables.
--
-- Basketball is split into men's/women's rather than one "college
-- basketball" value: Ticketmaster's own classification data doesn't
-- distinguish them (both are `genre=Basketball, subGenre=College`, no
-- structured gender field -- confirmed live during this feature's
-- design), so gating has to fall back to a "women" keyword check on the
-- event's own name (see apps/api/src/sports/types.rs). Keeping men's and
-- women's as distinct league values keeps that gender resolution a single
-- decision made once at gating time, rather than something every
-- downstream match/standings lookup has to re-derive.
--
-- `ALTER TYPE ... ADD VALUE` is safe to run as a normal migration on
-- Postgres 12+: it only needs to not be *used* in the same transaction it
-- was added in, which this migration doesn't do.
alter type sports_league add value 'ncaa_football';
alter type sports_league add value 'ncaa_mens_basketball';
alter type sports_league add value 'ncaa_womens_basketball';
