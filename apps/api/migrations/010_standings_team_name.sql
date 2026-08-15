-- Stores each team's display name alongside its cached standing, so the
-- detail view can show the *whole* conference/division table (every team
-- in the same group_name, ordered by standing_position) instead of just
-- the requesting team's own bare position number -- a number with no
-- surrounding context turned out not to be very useful on its own.
--
-- `sports_team_standings` is pure cache (see migrations/
-- 007_sports_enrichment.sql's docs -- nothing reads a past standing, only
-- "as of now"), so truncating it here is safe: every row gets refetched
-- and rewritten with a real team_name on the next standings refresh
-- anyway, same as any other stale-cache miss.
truncate table sports_team_standings;
alter table sports_team_standings add column team_name text not null;
