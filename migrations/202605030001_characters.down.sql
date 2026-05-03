drop trigger if exists character_warcraftlogs_snapshots_set_updated_at
    on character_warcraftlogs_snapshots;

drop index if exists idx_character_warcraftlogs_snapshots_fetched_at;
drop index if exists idx_character_warcraftlogs_snapshots_warcraftlogs_character_id;

drop table if exists character_warcraftlogs_snapshots;

drop trigger if exists character_battlenet_snapshots_set_updated_at
    on character_battlenet_snapshots;

drop index if exists idx_character_battlenet_snapshots_fetched_at;
drop index if exists idx_character_battlenet_snapshots_battlenet_character_id;

drop table if exists character_battlenet_snapshots;

drop trigger if exists characters_set_updated_at on characters;

drop index if exists idx_characters_identity_lookup;
drop index if exists idx_characters_user_id;
drop index if exists idx_characters_active_identity;

drop table if exists characters;
