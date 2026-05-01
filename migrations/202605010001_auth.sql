create extension if not exists pgcrypto;

create or replace function set_updated_at()
returns trigger as $$
begin
    new.updated_at = now();
    return new;
end;
$$ language plpgsql;

create table users (
    id uuid primary key default gen_random_uuid(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz null
);

create table discord_identities (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null unique references users(id) on delete restrict,
    discord_user_id text not null unique,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    last_login_at timestamptz null,
    last_profile_refresh_at timestamptz null,
    relogin_required_at timestamptz null,
    relogin_reason text null
);

create table discord_profiles (
    id uuid primary key default gen_random_uuid(),
    discord_identity_id uuid not null unique references discord_identities(id) on delete restrict,
    username text null,
    discriminator text null,
    global_name text null,
    avatar text null,
    bot boolean null,
    system boolean null,
    mfa_enabled boolean null,
    banner text null,
    accent_color integer null,
    locale text null,
    verified boolean null,
    flags integer null,
    premium_type integer null,
    public_flags integer null,
    avatar_decoration_data jsonb null,
    collectibles jsonb null,
    primary_guild jsonb null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table discord_oauth_tokens (
    id uuid primary key default gen_random_uuid(),
    discord_identity_id uuid not null unique references discord_identities(id) on delete restrict,
    access_token_ciphertext text not null,
    refresh_token_ciphertext text not null,
    token_type text not null,
    scope text not null,
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table user_sessions (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete restrict,
    token_hash text not null unique,
    csrf_token_hash text not null unique,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    last_seen_at timestamptz not null default now(),
    expires_at timestamptz not null,
    revoked_at timestamptz null,
    login_ip_address inet null,
    login_location_country text null,
    login_location_region text null,
    login_location_city text null,
    current_ip_address inet null,
    current_location_country text null,
    current_location_region text null,
    current_location_city text null,
    user_agent text null
);

create table oauth_states (
    id uuid primary key default gen_random_uuid(),
    state_hash text not null unique,
    provider text not null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null,
    consumed_at timestamptz null,
    ip_address inet null,
    user_agent text null
);

create table security_audit_events (
    id uuid primary key default gen_random_uuid(),
    user_id uuid null references users(id) on delete restrict,
    session_id uuid null references user_sessions(id) on delete restrict,
    event_type text not null,
    created_at timestamptz not null default now(),
    ip_address inet null,
    user_agent text null,
    metadata jsonb not null default '{}'::jsonb
);

create index idx_users_deleted_at on users(deleted_at);
create index idx_discord_identities_discord_user_id on discord_identities(discord_user_id);
create index idx_user_sessions_user_id_expires_at on user_sessions(user_id, expires_at);
create index idx_user_sessions_active on user_sessions(user_id, last_seen_at desc)
    where revoked_at is null;
create index idx_oauth_states_state_hash on oauth_states(state_hash);
create index idx_security_audit_events_user_id_created_at
    on security_audit_events(user_id, created_at desc);

create trigger users_set_updated_at
before update on users
for each row execute function set_updated_at();

create trigger discord_identities_set_updated_at
before update on discord_identities
for each row execute function set_updated_at();

create trigger discord_profiles_set_updated_at
before update on discord_profiles
for each row execute function set_updated_at();

create trigger discord_oauth_tokens_set_updated_at
before update on discord_oauth_tokens
for each row execute function set_updated_at();

create trigger user_sessions_set_updated_at
before update on user_sessions
for each row execute function set_updated_at();
