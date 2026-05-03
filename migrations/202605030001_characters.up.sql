create table characters (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete restrict,
    name text not null,
    region text not null,
    server text not null,
    class text not null,
    race text not null,
    spec text not null,
    role text not null,
    rank text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz null,
    constraint characters_name_not_blank check (length(trim(name)) > 0),
    constraint characters_region_not_blank check (length(trim(region)) > 0),
    constraint characters_server_not_blank check (length(trim(server)) > 0),
    constraint characters_class_not_blank check (length(trim(class)) > 0),
    constraint characters_race_not_blank check (length(trim(race)) > 0),
    constraint characters_spec_not_blank check (length(trim(spec)) > 0),
    constraint characters_role_valid check (role in ('tank', 'heal', 'melee', 'ranged')),
    constraint characters_rank_valid check (rank in ('main', 'alt'))
);

create unique index idx_characters_active_identity
    on characters(lower(trim(name)), lower(trim(region)), lower(trim(server)))
    where deleted_at is null;

create index idx_characters_user_id
    on characters(user_id, created_at desc)
    where deleted_at is null;

create index idx_characters_identity_lookup
    on characters(lower(trim(region)), lower(trim(server)), lower(trim(name)))
    where deleted_at is null;

create trigger characters_set_updated_at
before update on characters
for each row execute function set_updated_at();

create table character_battlenet_snapshots (
    id uuid primary key default gen_random_uuid(),
    character_id uuid not null unique references characters(id) on delete restrict,
    battlenet_character_id bigint null,
    race_id integer null,
    race_name text null,
    class_id integer null,
    class_name text null,
    gender text null,
    faction text null,
    level integer null,
    active_spec_id integer null,
    active_spec_name text null,
    average_item_level integer null,
    equipped_item_level integer null,
    avatar_url text null,
    main_raw_url text null,
    fetched_at timestamptz null,
    error text null,
    raw jsonb null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint character_battlenet_snapshots_gender_valid
        check (gender is null or gender in ('male', 'female')),
    constraint character_battlenet_snapshots_faction_valid
        check (faction is null or faction in ('alliance', 'horde')),
    constraint character_battlenet_snapshots_level_valid
        check (level is null or level >= 0),
    constraint character_battlenet_snapshots_average_item_level_valid
        check (average_item_level is null or average_item_level >= 0),
    constraint character_battlenet_snapshots_equipped_item_level_valid
        check (equipped_item_level is null or equipped_item_level >= 0),
    constraint character_battlenet_snapshots_error_not_blank
        check (error is null or length(trim(error)) > 0)
);

create index idx_character_battlenet_snapshots_battlenet_character_id
    on character_battlenet_snapshots(battlenet_character_id)
    where battlenet_character_id is not null;

create index idx_character_battlenet_snapshots_fetched_at
    on character_battlenet_snapshots(fetched_at desc)
    where fetched_at is not null;

create trigger character_battlenet_snapshots_set_updated_at
before update on character_battlenet_snapshots
for each row execute function set_updated_at();

create table character_warcraftlogs_snapshots (
    id uuid primary key default gen_random_uuid(),
    character_id uuid not null unique references characters(id) on delete restrict,
    warcraftlogs_character_id bigint null,
    gear jsonb null,
    rankings jsonb null,
    best_performance_average double precision null,
    median_performance_average double precision null,
    total_kills integer not null default 0,
    fetched_at timestamptz null,
    error text null,
    partial boolean not null default false,
    raw jsonb null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint character_warcraftlogs_snapshots_best_performance_average_valid
        check (
            best_performance_average is null
            or (best_performance_average >= 0 and best_performance_average <= 100)
        ),
    constraint character_warcraftlogs_snapshots_median_performance_average_valid
        check (
            median_performance_average is null
            or (median_performance_average >= 0 and median_performance_average <= 100)
        ),
    constraint character_warcraftlogs_snapshots_total_kills_valid
        check (total_kills >= 0),
    constraint character_warcraftlogs_snapshots_error_not_blank
        check (error is null or length(trim(error)) > 0)
);

create index idx_character_warcraftlogs_snapshots_warcraftlogs_character_id
    on character_warcraftlogs_snapshots(warcraftlogs_character_id)
    where warcraftlogs_character_id is not null;

create index idx_character_warcraftlogs_snapshots_fetched_at
    on character_warcraftlogs_snapshots(fetched_at desc)
    where fetched_at is not null;

create trigger character_warcraftlogs_snapshots_set_updated_at
before update on character_warcraftlogs_snapshots
for each row execute function set_updated_at();
