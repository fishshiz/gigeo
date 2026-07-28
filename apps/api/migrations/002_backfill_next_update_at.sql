-- Backfill next_update_at for playlists created before the periodic
-- updater existed. Spread from each row's own created_at + cadence
-- rather than now(), so existing playlists don't all become due on the
-- same first scheduler tick after deploy.
update playlist
set next_update_at = created_at + make_interval(days => update_cadence_days)
where next_update_at is null;
