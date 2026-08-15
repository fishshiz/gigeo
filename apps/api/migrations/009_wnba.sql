-- Extends sports enrichment to the WNBA, alongside the other pro majors.
-- Purely additive, same as migrations/008_ncaa_sports.sql.
--
-- Unlike college basketball, Ticketmaster tags WNBA events with their own
-- distinct subGenre value ("WNBA", separate from "Basketball"/"College")
-- -- confirmed live -- so this slots into the same subGenre-alone
-- detection path as NBA/NFL/NHL/MLB, not the genre+subGenre combo college
-- needs.
alter type sports_league add value 'wnba';
