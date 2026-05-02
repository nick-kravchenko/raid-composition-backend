drop trigger if exists guild_invites_set_updated_at on guild_invites;
drop trigger if exists guild_memberships_set_updated_at on guild_memberships;
drop trigger if exists guilds_set_updated_at on guilds;

drop index if exists idx_guilds_active;
drop index if exists idx_guild_invites_guild_id;
drop index if exists idx_guild_invites_active_code;
drop index if exists idx_guild_memberships_guild_id_role;
drop index if exists idx_guild_memberships_user_id;
drop index if exists idx_guild_memberships_active_user_guild;

drop table if exists guild_invites;
drop table if exists guild_memberships;
drop table if exists guilds;
