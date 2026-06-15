alter table spotify_account
add column spotify_user_id text unique;

alter table spotify_account
add column scope text;