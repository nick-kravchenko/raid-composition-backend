create table guilds (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    realm text not null,
    region text not null,
    faction text not null,
    game_version text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz null,
    constraint guilds_name_not_blank check (length(trim(name)) > 0),
    constraint guilds_realm_not_blank check (length(trim(realm)) > 0),
    constraint guilds_region_valid check (region in ('us', 'eu', 'kr', 'tw', 'cn')),
    constraint guilds_faction_valid check (faction in ('alliance', 'horde')),
    constraint guilds_game_version_valid check (game_version in ('classic1x', 'classic', 'classicann'))
);

create table guild_memberships (
    id uuid primary key default gen_random_uuid(),
    guild_id uuid not null references guilds(id) on delete restrict,
    user_id uuid not null references users(id) on delete restrict,
    role text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz null,
    constraint guild_memberships_role_valid check (role in ('applicant', 'raider', 'officer', 'admin'))
);

create table guild_invites (
    id uuid primary key default gen_random_uuid(),
    guild_id uuid not null references guilds(id) on delete restrict,
    code text not null,
    created_by_user_id uuid not null references users(id) on delete restrict,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    revoked_at timestamptz null,
    constraint guild_invites_code_not_blank check (length(trim(code)) > 0)
);

create unique index idx_guild_memberships_active_user_guild
    on guild_memberships(guild_id, user_id)
    where deleted_at is null;

create index idx_guild_memberships_user_id
    on guild_memberships(user_id)
    where deleted_at is null;

create index idx_guild_memberships_guild_id_role
    on guild_memberships(guild_id, role)
    where deleted_at is null;

create unique index idx_guild_invites_active_code
    on guild_invites(code)
    where revoked_at is null;

create index idx_guild_invites_guild_id
    on guild_invites(guild_id)
    where revoked_at is null;

create index idx_guilds_active
    on guilds(created_at desc)
    where deleted_at is null;

create trigger guilds_set_updated_at
before update on guilds
for each row execute function set_updated_at();

create trigger guild_memberships_set_updated_at
before update on guild_memberships
for each row execute function set_updated_at();

create trigger guild_invites_set_updated_at
before update on guild_invites
for each row execute function set_updated_at();
