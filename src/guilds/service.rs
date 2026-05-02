use actix_web::{HttpRequest, HttpResponse, web};
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::{crypto, discord::avatar_url, service as auth_service},
    guilds::dto::{
        CreateGuildRequestDto, GameVersion, GuildDto, GuildInviteDto, GuildInviteResponseDto,
        GuildMemberDiscordDto, GuildMemberDto, GuildMembersResponseDto, GuildResponseDto,
        GuildRole, GuildsResponseDto, UpdateGuildMemberRoleRequestDto, UpdateGuildRequestDto,
    },
    state::AppState,
};

const INVITE_TOKEN_BYTES: usize = 24;

#[derive(Debug, FromRow)]
struct GuildRow {
    id: Uuid,
    name: String,
    realm: String,
    region: String,
    faction: String,
    game_version: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    membership_role: String,
}

#[derive(Debug, FromRow)]
struct MembershipRow {
    role: String,
}

#[derive(Debug, FromRow)]
struct GuildMemberRow {
    user_id: Uuid,
    role: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    discord_user_id: String,
    discord_username: Option<String>,
    discord_global_name: Option<String>,
    discord_avatar: Option<String>,
    discord_discriminator: Option<String>,
}

#[derive(Debug, FromRow)]
struct InviteGuildRow {
    guild_id: Uuid,
}

#[derive(Debug, FromRow)]
struct GuildInviteRow {
    code: String,
}

pub async fn create_guild_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: CreateGuildRequestDto,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;
    validate_guild_fields(
        &payload.name,
        &payload.realm,
        &payload.region,
        &payload.faction,
        &payload.game_version,
    )?;

    let mut transaction = state.db_pool.begin().await?;
    let guild = sqlx::query_as::<_, GuildRow>(
        r#"
        with inserted_guild as (
            insert into guilds (name, realm, region, faction, game_version)
            values ($1, $2, $3, $4, $5)
            returning id, name, realm, region, faction, game_version, created_at, updated_at, deleted_at
        ),
        inserted_membership as (
            insert into guild_memberships (guild_id, user_id, role)
            select id, $6, 'admin' from inserted_guild
            returning guild_id, role
        )
        select g.id, g.name, g.realm, g.region, g.faction, g.game_version,
               g.created_at, g.updated_at, g.deleted_at, m.role as membership_role
        from inserted_guild g
        join inserted_membership m on m.guild_id = g.id
        "#,
    )
    .bind(payload.name.trim())
    .bind(payload.realm.trim())
    .bind(payload.region.trim().to_ascii_lowercase())
    .bind(payload.faction.trim().to_ascii_lowercase())
    .bind(payload.game_version.trim().to_ascii_lowercase())
    .bind(context.user_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(json_response(
        HttpResponse::Created(),
        refreshed_cookie,
        GuildResponseDto {
            guild: guild_row_to_dto(guild, None)?,
        },
    ))
}

pub async fn list_guilds_response(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    let guilds = sqlx::query_as::<_, GuildRow>(
        r#"
        select g.id, g.name, g.realm, g.region, g.faction, g.game_version,
               g.created_at, g.updated_at, g.deleted_at, gm.role as membership_role
        from guilds g
        join guild_memberships gm on gm.guild_id = g.id
        where gm.user_id = $1
          and gm.deleted_at is null
          and g.deleted_at is null
        order by g.created_at desc
        "#,
    )
    .bind(context.user_id)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        GuildsResponseDto {
            guilds: guilds
                .into_iter()
                .map(|guild| guild_row_to_dto(guild, None))
                .collect::<Result<Vec<_>, _>>()?,
        },
    ))
}

pub async fn get_guild_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    let guild = load_visible_guild(&state.db_pool, guild_id, context.user_id).await?;
    let invite_url = active_invite_url_for_guild(
        &state.db_pool,
        guild_id,
        &state.config.frontend.base_url,
        &guild.membership_role,
    )
    .await?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        GuildResponseDto {
            guild: guild_row_to_dto(guild, invite_url)?,
        },
    ))
}

pub async fn update_guild_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
    payload: UpdateGuildRequestDto,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;
    require_admin(&state.db_pool, guild_id, context.user_id).await?;

    let current = load_visible_guild(&state.db_pool, guild_id, context.user_id).await?;
    let name = payload.name.as_deref().unwrap_or(&current.name);
    let realm = payload.realm.as_deref().unwrap_or(&current.realm);
    let region = payload.region.as_deref().unwrap_or(&current.region);
    let faction = payload.faction.as_deref().unwrap_or(&current.faction);
    let game_version = payload
        .game_version
        .as_deref()
        .unwrap_or(&current.game_version);
    validate_guild_fields(name, realm, region, faction, game_version)?;

    let guild = sqlx::query_as::<_, GuildRow>(
        r#"
        update guilds
        set name = $2,
            realm = $3,
            region = $4,
            faction = $5,
            game_version = $6
        where id = $1
          and deleted_at is null
        returning id, name, realm, region, faction, game_version, created_at, updated_at, deleted_at,
                  'admin'::text as membership_role
        "#,
    )
    .bind(guild_id)
    .bind(name.trim())
    .bind(realm.trim())
    .bind(region.trim().to_ascii_lowercase())
    .bind(faction.trim().to_ascii_lowercase())
    .bind(game_version.trim().to_ascii_lowercase())
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(guild_not_found)?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        GuildResponseDto {
            guild: guild_row_to_dto(guild, None)?,
        },
    ))
}

pub async fn delete_guild_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;
    require_admin(&state.db_pool, guild_id, context.user_id).await?;

    let result = sqlx::query(
        r#"
        update guilds
        set deleted_at = now()
        where id = $1
          and deleted_at is null
        "#,
    )
    .bind(guild_id)
    .execute(&state.db_pool)
    .await?;

    if result.rows_affected() != 1 {
        return Err(guild_not_found());
    }

    let mut response = HttpResponse::NoContent();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response.finish())
}

pub async fn create_invite_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;
    require_admin(&state.db_pool, guild_id, context.user_id).await?;

    let code = create_invite(&state.db_pool, guild_id, context.user_id).await?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        GuildInviteResponseDto {
            invite: GuildInviteDto { guild_id, code },
        },
    ))
}

pub async fn accept_invite_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    invite_code: String,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;
    let invite = sqlx::query_as::<_, InviteGuildRow>(
        r#"
        select gi.guild_id
        from guild_invites gi
        join guilds g on g.id = gi.guild_id
        where gi.code = $1
          and gi.revoked_at is null
          and g.deleted_at is null
        "#,
    )
    .bind(invite_code.trim())
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| ApiError::not_found("guild.invite_not_found", "Guild invite was not found."))?;

    let existing = sqlx::query_scalar::<_, Uuid>(
        r#"
        select id
        from guild_memberships
        where guild_id = $1
          and user_id = $2
          and deleted_at is null
        "#,
    )
    .bind(invite.guild_id)
    .bind(context.user_id)
    .fetch_optional(&state.db_pool)
    .await?;

    if existing.is_some() {
        return Err(ApiError::conflict(
            "guild.already_member",
            "User is already a guild member.",
        ));
    }

    sqlx::query(
        r#"
        insert into guild_memberships (guild_id, user_id, role)
        values ($1, $2, 'applicant')
        "#,
    )
    .bind(invite.guild_id)
    .bind(context.user_id)
    .execute(&state.db_pool)
    .await?;

    let guild = load_visible_guild(&state.db_pool, invite.guild_id, context.user_id).await?;
    Ok(json_response(
        HttpResponse::Created(),
        refreshed_cookie,
        GuildResponseDto {
            guild: guild_row_to_dto(guild, None)?,
        },
    ))
}

pub async fn list_members_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    require_membership(&state.db_pool, guild_id, context.user_id).await?;

    let members = sqlx::query_as::<_, GuildMemberRow>(
        r#"
        select
            gm.user_id,
            gm.role,
            gm.created_at,
            gm.updated_at,
            di.discord_user_id,
            dp.username as discord_username,
            dp.global_name as discord_global_name,
            dp.avatar as discord_avatar,
            dp.discriminator as discord_discriminator
        from guild_memberships gm
        join discord_identities di on di.user_id = gm.user_id
        left join discord_profiles dp on dp.discord_identity_id = di.id
        where gm.guild_id = $1
          and gm.deleted_at is null
        order by gm.created_at asc
        "#,
    )
    .bind(guild_id)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        GuildMembersResponseDto {
            members: members
                .into_iter()
                .map(member_row_to_dto)
                .collect::<Result<Vec<_>, _>>()?,
        },
    ))
}

pub async fn update_member_role_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    guild_id: Uuid,
    user_id: Uuid,
    payload: UpdateGuildMemberRoleRequestDto,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = auth_service::authenticate(&state, &req).await?;
    auth_service::validate_csrf_async(&state, &req, context.session_id).await?;
    require_admin(&state.db_pool, guild_id, context.user_id).await?;

    if !payload.role.is_allowed_promotion_target() {
        return Err(ApiError::bad_request(
            "guild.invalid_role",
            "Guild member role is invalid for this operation.",
        ));
    }

    let member = sqlx::query_as::<_, GuildMemberRow>(
        r#"
        with updated_membership as (
            update guild_memberships
            set role = $3
            where guild_id = $1
              and user_id = $2
              and role = 'applicant'
              and deleted_at is null
            returning user_id, role, created_at, updated_at
        )
        select
            um.user_id,
            um.role,
            um.created_at,
            um.updated_at,
            di.discord_user_id,
            dp.username as discord_username,
            dp.global_name as discord_global_name,
            dp.avatar as discord_avatar,
            dp.discriminator as discord_discriminator
        from updated_membership um
        join discord_identities di on di.user_id = um.user_id
        left join discord_profiles dp on dp.discord_identity_id = di.id
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .bind(payload.role.as_str())
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| {
        ApiError::not_found(
            "guild.member_not_found",
            "Guild applicant was not found for promotion.",
        )
    })?;

    Ok(json_response(
        HttpResponse::Ok(),
        refreshed_cookie,
        member_row_to_dto(member)?,
    ))
}

pub fn validate_guild_fields(
    name: &str,
    realm: &str,
    region: &str,
    faction: &str,
    game_version: &str,
) -> Result<(), ApiError> {
    if name.trim().is_empty()
        || realm.trim().is_empty()
        || !matches!(
            region.trim().to_ascii_lowercase().as_str(),
            "us" | "eu" | "kr" | "tw" | "cn"
        )
        || !matches!(
            faction.trim().to_ascii_lowercase().as_str(),
            "alliance" | "horde"
        )
        || !matches!(
            game_version.trim().to_ascii_lowercase().as_str(),
            "classic1x" | "classic" | "classicann"
        )
    {
        return Err(ApiError::bad_request(
            "guild.validation_failed",
            "Guild payload is invalid.",
        ));
    }

    Ok(())
}

async fn create_invite(pool: &PgPool, guild_id: Uuid, user_id: Uuid) -> Result<String, ApiError> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        r#"
        update guild_invites
        set revoked_at = now()
        where guild_id = $1
          and revoked_at is null
        "#,
    )
    .bind(guild_id)
    .execute(&mut *transaction)
    .await?;

    let code = crypto::random_token(INVITE_TOKEN_BYTES);
    sqlx::query(
        r#"
        insert into guild_invites (guild_id, code, created_by_user_id)
        values ($1, $2, $3)
        "#,
    )
    .bind(guild_id)
    .bind(&code)
    .bind(user_id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    Ok(code)
}

async fn active_invite_url_for_guild(
    pool: &PgPool,
    guild_id: Uuid,
    frontend_base_url: &str,
    membership_role: &str,
) -> Result<Option<String>, ApiError> {
    let role = parse_role(membership_role)?;
    if !matches!(role, GuildRole::Admin | GuildRole::Officer) {
        return Ok(None);
    }

    let invite = sqlx::query_as::<_, GuildInviteRow>(
        r#"
        select code
        from guild_invites
        where guild_id = $1
          and revoked_at is null
        order by created_at desc
        limit 1
        "#,
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await?;

    Ok(invite.and_then(|invite| invite_url_for_role(frontend_base_url, &invite.code, role)))
}

async fn load_visible_guild(
    pool: &PgPool,
    guild_id: Uuid,
    user_id: Uuid,
) -> Result<GuildRow, ApiError> {
    sqlx::query_as::<_, GuildRow>(
        r#"
        select g.id, g.name, g.realm, g.region, g.faction, g.game_version,
               g.created_at, g.updated_at, g.deleted_at, gm.role as membership_role
        from guilds g
        join guild_memberships gm on gm.guild_id = g.id
        where g.id = $1
          and gm.user_id = $2
          and gm.deleted_at is null
          and g.deleted_at is null
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(guild_not_found)
}

async fn require_admin(pool: &PgPool, guild_id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    let role = require_membership(pool, guild_id, user_id).await?;
    if role != GuildRole::Admin {
        return Err(ApiError::forbidden(
            "guild.admin_required",
            "Guild admin permission is required.",
        ));
    }

    Ok(())
}

async fn require_membership(
    pool: &PgPool,
    guild_id: Uuid,
    user_id: Uuid,
) -> Result<GuildRole, ApiError> {
    let membership = sqlx::query_as::<_, MembershipRow>(
        r#"
        select gm.role
        from guild_memberships gm
        join guilds g on g.id = gm.guild_id
        where gm.guild_id = $1
          and gm.user_id = $2
          and gm.deleted_at is null
          and g.deleted_at is null
        "#,
    )
    .bind(guild_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(guild_not_found)?;

    parse_role(&membership.role)
}

fn guild_row_to_dto(row: GuildRow, invite_url: Option<String>) -> Result<GuildDto, ApiError> {
    Ok(GuildDto {
        id: row.id,
        name: row.name,
        realm: row.realm,
        region: row.region,
        faction: row.faction,
        game_version: parse_game_version(&row.game_version)?,
        invite_url,
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
        membership_role: parse_role(&row.membership_role)?,
    })
}

fn member_row_to_dto(row: GuildMemberRow) -> Result<GuildMemberDto, ApiError> {
    Ok(GuildMemberDto {
        user_id: row.user_id,
        role: parse_role(&row.role)?,
        discord: GuildMemberDiscordDto {
            id: row.discord_user_id.clone(),
            username: row.discord_username,
            global_name: row.discord_global_name,
            avatar_url: avatar_url(
                &row.discord_user_id,
                row.discord_avatar.as_deref(),
                row.discord_discriminator.as_deref(),
            ),
        },
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn parse_role(value: &str) -> Result<GuildRole, ApiError> {
    value.parse().map_err(|_| ApiError::internal())
}

fn parse_game_version(value: &str) -> Result<GameVersion, ApiError> {
    value.parse().map_err(|_| ApiError::internal())
}

fn invite_url_for_role(
    frontend_base_url: &str,
    invite_code: &str,
    role: GuildRole,
) -> Option<String> {
    if !matches!(role, GuildRole::Admin | GuildRole::Officer) {
        return None;
    }

    Some(format!(
        "{}/guild-invites/{}",
        frontend_base_url.trim_end_matches('/'),
        invite_code
    ))
}

fn guild_not_found() -> ApiError {
    ApiError::not_found("guild.not_found", "Guild was not found.")
}

fn json_response<T: serde::Serialize>(
    mut builder: actix_web::HttpResponseBuilder,
    refreshed_cookie: Option<actix_web::cookie::Cookie<'static>>,
    payload: T,
) -> HttpResponse {
    if let Some(cookie) = refreshed_cookie {
        builder.cookie(cookie);
    }
    builder.json(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_guild_fields_rejects_blank_text() {
        let error =
            validate_guild_fields(" ", "Draenor", "eu", "horde", "classic").expect_err("invalid");

        assert_eq!(error.code, "guild.validation_failed");
    }

    #[test]
    fn validate_guild_fields_accepts_supported_region_and_faction() {
        validate_guild_fields("Raid Team", "Draenor", "eu", "horde", "classic1x").expect("valid");
        validate_guild_fields("Raid Team", "Draenor", "eu", "horde", "classic").expect("valid");
        validate_guild_fields("Raid Team", "Draenor", "eu", "horde", "classicann").expect("valid");
    }

    #[test]
    fn validate_guild_fields_rejects_unknown_region() {
        let error = validate_guild_fields("Raid Team", "Draenor", "moon", "horde", "classic")
            .expect_err("invalid");

        assert_eq!(error.code, "guild.validation_failed");
    }

    #[test]
    fn validate_guild_fields_rejects_unknown_faction() {
        let error = validate_guild_fields("Raid Team", "Draenor", "eu", "neutral", "classic")
            .expect_err("invalid");

        assert_eq!(error.code, "guild.validation_failed");
    }

    #[test]
    fn validate_guild_fields_rejects_unknown_game_version() {
        let error = validate_guild_fields("Raid Team", "Draenor", "eu", "horde", "retail")
            .expect_err("invalid");

        assert_eq!(error.code, "guild.validation_failed");
    }

    #[test]
    fn invite_url_is_visible_to_admins_and_officers_only() {
        assert_eq!(
            invite_url_for_role("https://app.example", "invite-code", GuildRole::Admin),
            Some("https://app.example/guild-invites/invite-code".to_string())
        );
        assert_eq!(
            invite_url_for_role("https://app.example", "invite-code", GuildRole::Officer),
            Some("https://app.example/guild-invites/invite-code".to_string())
        );
        assert_eq!(
            invite_url_for_role("https://app.example", "invite-code", GuildRole::Raider),
            None
        );
        assert_eq!(
            invite_url_for_role("https://app.example", "invite-code", GuildRole::Applicant),
            None
        );
    }

    #[test]
    fn invite_url_trims_frontend_base_url_trailing_slash() {
        assert_eq!(
            invite_url_for_role("https://app.example/", "invite-code", GuildRole::Admin),
            Some("https://app.example/guild-invites/invite-code".to_string())
        );
    }

    #[test]
    fn member_row_to_dto_includes_discord_data() {
        let member = GuildMemberRow {
            user_id: Uuid::nil(),
            role: "raider".to_string(),
            created_at: DateTime::<Utc>::default(),
            updated_at: DateTime::<Utc>::default(),
            discord_user_id: "123456789".to_string(),
            discord_username: Some("thrall".to_string()),
            discord_global_name: Some("Thrall".to_string()),
            discord_avatar: Some("avatar_hash".to_string()),
            discord_discriminator: Some("0001".to_string()),
        };

        let dto = member_row_to_dto(member).expect("dto");

        assert_eq!(dto.discord.id, "123456789");
        assert_eq!(dto.discord.username.as_deref(), Some("thrall"));
        assert_eq!(dto.discord.global_name.as_deref(), Some("Thrall"));
        assert_eq!(
            dto.discord.avatar_url,
            "https://cdn.discordapp.com/avatars/123456789/avatar_hash.png?size=128"
        );
    }
}
