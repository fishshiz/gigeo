alter table playlist add column deleted_at timestamptz;

-- allow re-adding the same spotify playlist after a soft-delete
alter table playlist drop constraint playlist_account_id_provider_playlist_id_key;
create unique index playlist_account_provider_active_idx
    on playlist (account_id, provider_playlist_id)
    where deleted_at is null;

-- exclude soft-deleted rows from the background updater's claim query
drop index playlist_scheduler_idx;
create index playlist_scheduler_idx
    on playlist (next_update_at)
    where is_active and deleted_at is null;

create index playlist_account_active_idx on playlist (account_id) where deleted_at is null;
