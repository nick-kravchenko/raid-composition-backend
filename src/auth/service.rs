use std::net::IpAddr;

use actix_web::{
    HttpRequest, HttpResponse,
    cookie::{Cookie, time::Duration as CookieDuration},
    http::header,
    web,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::{
    api::error::ApiError,
    auth::{
        crypto::{self, HashDomain},
        discord::{DiscordClient, DiscordUserProfile, OAuthTokens, avatar_url},
        geoip::Location,
    },
    config::{Config, CookieConfig},
    state::AppState,
};

const SESSION_TTL_DAYS: i64 = 30;
const OAUTH_STATE_TTL_MINUTES: i64 = 10;
const SESSION_REFRESH_THROTTLE_MINUTES: i64 = 5;

#[derive(Debug, Serialize)]
pub struct DiscordAuthUrlResponseDto {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct DiscordCallbackRequestDto {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct AuthSessionResponseDto {
    pub user: SafeUserDto,
    pub session: SafeSessionDto,
}

#[derive(Debug, Serialize)]
pub struct SessionsResponseDto {
    pub sessions: Vec<SafeSessionDto>,
}

#[derive(Debug, Serialize)]
pub struct SafeUserDto {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub discord: SafeDiscordUserDto,
}

#[derive(Debug, Serialize)]
pub struct SafeDiscordUserDto {
    pub id: String,
    pub username: Option<String>,
    pub global_name: Option<String>,
    pub avatar_url: String,
}

#[derive(Debug, Serialize)]
pub struct SafeSessionDto {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub login_location: LocationDto,
    pub current_location: LocationDto,
    pub is_current: bool,
}

#[derive(Debug, Serialize)]
pub struct LocationDto {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub user_created_at: DateTime<Utc>,
    pub session_id: Uuid,
    pub session_created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub login_location_country: Option<String>,
    pub login_location_region: Option<String>,
    pub login_location_city: Option<String>,
    pub current_location_country: Option<String>,
    pub current_location_region: Option<String>,
    pub current_location_city: Option<String>,
    pub discord_user_id: String,
    pub username: Option<String>,
    pub global_name: Option<String>,
    pub avatar: Option<String>,
    pub discriminator: Option<String>,
}

#[derive(Debug, FromRow)]
struct SessionStatusRow {
    revoked_at: Option<DateTime<Utc>>,
    expires_at: DateTime<Utc>,
    deleted_at: Option<DateTime<Utc>>,
    relogin_required_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ExistingDiscordIdentity {
    identity_id: Uuid,
    user_id: Uuid,
    deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct SessionRow {
    id: Uuid,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    user_agent: Option<String>,
    login_location_country: Option<String>,
    login_location_region: Option<String>,
    login_location_city: Option<String>,
    current_location_country: Option<String>,
    current_location_region: Option<String>,
    current_location_city: Option<String>,
}

#[derive(Debug)]
pub struct LoginSession {
    pub session_token: String,
    pub csrf_token: String,
}

pub async fn get_discord_auth_url(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<web::Json<DiscordAuthUrlResponseDto>, ApiError> {
    let raw_state = crypto::random_token(32);
    let state_hash = crypto::token_hash(
        &state.config.security.session_hmac_secret,
        HashDomain::OAuthState,
        &raw_state,
    );
    let ip = request_ip(&req).map(|ip| ip.to_string());
    let user_agent = request_user_agent(&req);
    let expires_at = Utc::now() + Duration::minutes(OAUTH_STATE_TTL_MINUTES);

    sqlx::query(
        r#"
        insert into oauth_states (state_hash, provider, expires_at, ip_address, user_agent)
        values ($1, 'discord', $2, $3::inet, $4)
        "#,
    )
    .bind(&state_hash)
    .bind(expires_at)
    .bind(ip.as_deref())
    .bind(user_agent.as_deref())
    .execute(&state.db_pool)
    .await?;

    write_audit_event(
        &state.db_pool,
        None,
        None,
        "discord_oauth_started",
        ip.as_deref(),
        user_agent.as_deref(),
        json!({}),
    )
    .await?;

    Ok(web::Json(DiscordAuthUrlResponseDto {
        url: state.config.discord.authorization_url(&raw_state),
    }))
}

pub async fn handle_discord_callback(
    state: web::Data<AppState>,
    req: HttpRequest,
    payload: DiscordCallbackRequestDto,
) -> Result<HttpResponse, ApiError> {
    if payload.code.trim().is_empty() || payload.state.trim().is_empty() {
        return Err(ApiError::bad_request(
            "validation.invalid_request",
            "Request payload is invalid.",
        ));
    }

    let ip = request_ip(&req);
    let ip_string = ip.map(|value| value.to_string());
    let user_agent = request_user_agent(&req);

    consume_oauth_state(&state.db_pool, &state.config, &payload.state).await?;

    let discord_client = DiscordClient::new(state.http_client.clone());
    let tokens = discord_client
        .exchange_code(&state.config.discord, &payload.code)
        .await?;
    let profile = discord_client.fetch_user_profile(&tokens).await?;

    let access_token_ciphertext = crypto::encrypt_token(
        &state.config.security.discord_token_encryption_key,
        &tokens.access_token,
    )?;
    let refresh_token_ciphertext = crypto::encrypt_token(
        &state.config.security.discord_token_encryption_key,
        &tokens.refresh_token,
    )?;

    let location = state.geoip.lookup(ip);
    let login_session = persist_successful_login(
        &state.db_pool,
        &state.config,
        &profile,
        &tokens,
        &access_token_ciphertext,
        &refresh_token_ciphertext,
        ip_string.as_deref(),
        user_agent.as_deref(),
        &location,
    )
    .await?;

    let response = HttpResponse::NoContent()
        .cookie(session_cookie(
            &state.config.cookie,
            &login_session.session_token,
            false,
        ))
        .cookie(csrf_cookie(
            &state.config.cookie,
            &login_session.csrf_token,
            false,
        ))
        .finish();

    Ok(response)
}

pub async fn authenticate(
    state: &AppState,
    req: &HttpRequest,
) -> Result<(AuthContext, Option<Cookie<'static>>), ApiError> {
    let token = req
        .cookie(&state.config.cookie.session_name)
        .map(|cookie| cookie.value().to_string())
        .ok_or_else(|| ApiError::unauthorized("Authentication is required."))?;

    let token_hash = crypto::token_hash(
        &state.config.security.session_hmac_secret,
        HashDomain::Session,
        &token,
    );

    let mut context = load_auth_context(&state.db_pool, &token_hash).await?;

    let refreshed_cookie = if context.last_seen_at
        < Utc::now() - Duration::minutes(SESSION_REFRESH_THROTTLE_MINUTES)
    {
        let ip = request_ip(req);
        let ip_string = ip.map(|value| value.to_string());
        let location = state.geoip.lookup(ip);
        let refreshed_at = Utc::now();
        let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);

        sqlx::query(
            r#"
            update user_sessions
            set last_seen_at = now(),
                expires_at = $2,
                current_ip_address = $3::inet,
                current_location_country = $4,
                current_location_region = $5,
                current_location_city = $6
            where id = $1
            "#,
        )
        .bind(context.session_id)
        .bind(expires_at)
        .bind(ip_string.as_deref())
        .bind(location.country.as_deref())
        .bind(location.region.as_deref())
        .bind(location.city.as_deref())
        .execute(&state.db_pool)
        .await?;

        write_audit_event(
            &state.db_pool,
            Some(context.user_id),
            Some(context.session_id),
            "session_refreshed",
            ip_string.as_deref(),
            request_user_agent(req).as_deref(),
            json!({}),
        )
        .await?;

        context.last_seen_at = refreshed_at;
        context.expires_at = expires_at;
        context.current_location_country = location.country;
        context.current_location_region = location.region;
        context.current_location_city = location.city;

        Some(session_cookie(&state.config.cookie, &token, false))
    } else {
        None
    };

    Ok((context, refreshed_cookie))
}

async fn csrf_hash_for_session(pool: &PgPool, session_id: Uuid) -> Result<String, ApiError> {
    let row = sqlx::query("select csrf_token_hash from user_sessions where id = $1")
        .bind(session_id)
        .fetch_one(pool)
        .await?;
    Ok(row.try_get("csrf_token_hash")?)
}

pub async fn current_session_response(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = authenticate(&state, &req).await?;
    let mut response = HttpResponse::Ok();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response.json(AuthSessionResponseDto {
        user: safe_user(&context),
        session: safe_session_from_context(&context, true),
    }))
}

pub async fn list_sessions_response(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = authenticate(&state, &req).await?;
    let sessions = sqlx::query_as::<_, SessionRow>(
        r#"
        select id, created_at, last_seen_at, expires_at, user_agent,
               login_location_country, login_location_region, login_location_city,
               current_location_country, current_location_region, current_location_city
        from user_sessions
        where user_id = $1
          and revoked_at is null
          and expires_at > now()
        order by last_seen_at desc
        "#,
    )
    .bind(context.user_id)
    .fetch_all(&state.db_pool)
    .await?;

    let mut response = HttpResponse::Ok();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response.json(SessionsResponseDto {
        sessions: sessions
            .into_iter()
            .map(|session| safe_session_from_row(session, context.session_id))
            .collect(),
    }))
}

pub async fn logout_response(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (context, _) = authenticate(&state, &req).await?;
    validate_csrf_async(&state, &req, context.session_id).await?;

    sqlx::query("update user_sessions set revoked_at = now() where id = $1 and revoked_at is null")
        .bind(context.session_id)
        .execute(&state.db_pool)
        .await?;

    write_audit_event(
        &state.db_pool,
        Some(context.user_id),
        Some(context.session_id),
        "session_revoked",
        request_ip(&req).map(|ip| ip.to_string()).as_deref(),
        request_user_agent(&req).as_deref(),
        json!({"reason": "logout"}),
    )
    .await?;

    Ok(HttpResponse::NoContent()
        .cookie(session_cookie(&state.config.cookie, "", true))
        .cookie(csrf_cookie(&state.config.cookie, "", true))
        .finish())
}

pub async fn logout_all_other_sessions_response(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = authenticate(&state, &req).await?;
    validate_csrf_async(&state, &req, context.session_id).await?;

    sqlx::query(
        r#"
        update user_sessions
        set revoked_at = now()
        where user_id = $1
          and id <> $2
          and revoked_at is null
          and expires_at > now()
        "#,
    )
    .bind(context.user_id)
    .bind(context.session_id)
    .execute(&state.db_pool)
    .await?;

    write_audit_event(
        &state.db_pool,
        Some(context.user_id),
        Some(context.session_id),
        "sessions_revoked_except_current",
        request_ip(&req).map(|ip| ip.to_string()).as_deref(),
        request_user_agent(&req).as_deref(),
        json!({}),
    )
    .await?;

    let mut response = HttpResponse::NoContent();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response.finish())
}

pub async fn revoke_session_response(
    state: web::Data<AppState>,
    req: HttpRequest,
    session_id: Uuid,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = authenticate(&state, &req).await?;
    validate_csrf_async(&state, &req, context.session_id).await?;

    if session_id == context.session_id {
        return Err(ApiError::bad_request(
            "auth.current_session_delete_forbidden",
            "Current session cannot be deleted from this endpoint.",
        ));
    }

    sqlx::query(
        r#"
        update user_sessions
        set revoked_at = now()
        where id = $1
          and user_id = $2
          and revoked_at is null
          and expires_at > now()
        "#,
    )
    .bind(session_id)
    .bind(context.user_id)
    .execute(&state.db_pool)
    .await?;

    write_audit_event(
        &state.db_pool,
        Some(context.user_id),
        Some(session_id),
        "session_revoked",
        request_ip(&req).map(|ip| ip.to_string()).as_deref(),
        request_user_agent(&req).as_deref(),
        json!({"reason": "session_management"}),
    )
    .await?;

    let mut response = HttpResponse::NoContent();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response.finish())
}

pub async fn refresh_csrf_response(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let (context, refreshed_cookie) = authenticate(&state, &req).await?;
    let csrf_token = crypto::random_token(32);
    let csrf_hash = crypto::token_hash(
        &state.config.security.session_hmac_secret,
        HashDomain::Csrf,
        &csrf_token,
    );

    sqlx::query("update user_sessions set csrf_token_hash = $1 where id = $2")
        .bind(csrf_hash)
        .bind(context.session_id)
        .execute(&state.db_pool)
        .await?;

    let mut response = HttpResponse::NoContent();
    if let Some(cookie) = refreshed_cookie {
        response.cookie(cookie);
    }
    Ok(response
        .cookie(csrf_cookie(&state.config.cookie, &csrf_token, false))
        .finish())
}

async fn consume_oauth_state(pool: &PgPool, config: &Config, state: &str) -> Result<(), ApiError> {
    let state_hash = crypto::token_hash(
        &config.security.session_hmac_secret,
        HashDomain::OAuthState,
        state,
    );

    let row = sqlx::query(
        r#"
        select provider, expires_at, consumed_at
        from oauth_states
        where state_hash = $1
        "#,
    )
    .bind(&state_hash)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        return Err(ApiError::bad_request(
            "auth.invalid_oauth_state",
            "OAuth state is invalid or expired.",
        ));
    };

    let provider: String = row.try_get("provider")?;
    let expires_at: DateTime<Utc> = row.try_get("expires_at")?;
    let consumed_at: Option<DateTime<Utc>> = row.try_get("consumed_at")?;

    if provider != "discord" {
        return Err(ApiError::bad_request(
            "auth.invalid_oauth_state",
            "OAuth state is invalid or expired.",
        ));
    }

    if consumed_at.is_some() {
        return Err(ApiError::conflict(
            "auth.oauth_state_consumed",
            "OAuth state has already been used.",
        ));
    }

    if expires_at <= Utc::now() {
        return Err(ApiError::bad_request(
            "auth.oauth_state_expired",
            "OAuth state is invalid or expired.",
        ));
    }

    let result = sqlx::query(
        r#"
        update oauth_states
        set consumed_at = now()
        where state_hash = $1
          and consumed_at is null
          and expires_at > now()
          and provider = 'discord'
        "#,
    )
    .bind(&state_hash)
    .execute(pool)
    .await?;

    if result.rows_affected() != 1 {
        return Err(ApiError::conflict(
            "auth.oauth_state_consumed",
            "OAuth state has already been used.",
        ));
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn persist_successful_login(
    pool: &PgPool,
    config: &Config,
    profile: &DiscordUserProfile,
    tokens: &OAuthTokens,
    access_token_ciphertext: &str,
    refresh_token_ciphertext: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    location: &Location,
) -> Result<LoginSession, ApiError> {
    let session_token = crypto::random_token(32);
    let csrf_token = crypto::random_token(32);
    let session_hash = crypto::token_hash(
        &config.security.session_hmac_secret,
        HashDomain::Session,
        &session_token,
    );
    let csrf_hash = crypto::token_hash(
        &config.security.session_hmac_secret,
        HashDomain::Csrf,
        &csrf_token,
    );
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);

    let mut tx = pool.begin().await?;
    let (user_id, identity_id) = find_or_create_user(&mut tx, profile, ip, user_agent).await?;

    upsert_discord_profile(&mut tx, identity_id, profile).await?;
    upsert_discord_tokens(
        &mut tx,
        identity_id,
        tokens,
        access_token_ciphertext,
        refresh_token_ciphertext,
    )
    .await?;

    let session_id: Uuid = sqlx::query_scalar(
        r#"
        insert into user_sessions (
            user_id, token_hash, csrf_token_hash, expires_at,
            login_ip_address, login_location_country, login_location_region, login_location_city,
            current_ip_address, current_location_country, current_location_region, current_location_city,
            user_agent
        )
        values ($1, $2, $3, $4, $5::inet, $6, $7, $8, $5::inet, $6, $7, $8, $9)
        returning id
        "#,
    )
    .bind(user_id)
    .bind(&session_hash)
    .bind(&csrf_hash)
    .bind(expires_at)
    .bind(ip)
    .bind(location.country.as_deref())
    .bind(location.region.as_deref())
    .bind(location.city.as_deref())
    .bind(user_agent)
    .fetch_one(tx.as_mut())
    .await?;

    write_audit_event_tx(
        &mut tx,
        Some(user_id),
        Some(session_id),
        "discord_oauth_succeeded",
        ip,
        user_agent,
        json!({"discord_user_id": profile.id}),
    )
    .await?;
    write_audit_event_tx(
        &mut tx,
        Some(user_id),
        Some(session_id),
        "discord_profile_refreshed",
        ip,
        user_agent,
        json!({}),
    )
    .await?;
    write_audit_event_tx(
        &mut tx,
        Some(user_id),
        Some(session_id),
        "session_created",
        ip,
        user_agent,
        json!({}),
    )
    .await?;

    tx.commit().await?;

    Ok(LoginSession {
        session_token,
        csrf_token,
    })
}

async fn find_or_create_user(
    tx: &mut Transaction<'_, Postgres>,
    profile: &DiscordUserProfile,
    ip: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(Uuid, Uuid), ApiError> {
    let existing = sqlx::query_as::<_, ExistingDiscordIdentity>(
        r#"
        select di.id as identity_id, di.user_id, users.deleted_at
        from discord_identities di
        join users on users.id = di.user_id
        where di.discord_user_id = $1
        "#,
    )
    .bind(&profile.id)
    .fetch_optional(tx.as_mut())
    .await?;

    if let Some(existing) = existing {
        if existing.deleted_at.is_some() {
            sqlx::query("update users set deleted_at = null where id = $1")
                .bind(existing.user_id)
                .execute(tx.as_mut())
                .await?;
            write_audit_event_tx(
                tx,
                Some(existing.user_id),
                None,
                "user_restored",
                ip,
                user_agent,
                json!({}),
            )
            .await?;
        }

        sqlx::query(
            r#"
            update discord_identities
            set last_login_at = now(), last_profile_refresh_at = now()
            where id = $1
            "#,
        )
        .bind(existing.identity_id)
        .execute(tx.as_mut())
        .await?;

        return Ok((existing.user_id, existing.identity_id));
    }

    let user_id: Uuid = sqlx::query_scalar("insert into users default values returning id")
        .fetch_one(tx.as_mut())
        .await?;

    let identity_id: Uuid = sqlx::query_scalar(
        r#"
        insert into discord_identities (
            user_id, discord_user_id, last_login_at, last_profile_refresh_at
        )
        values ($1, $2, now(), now())
        returning id
        "#,
    )
    .bind(user_id)
    .bind(&profile.id)
    .fetch_one(tx.as_mut())
    .await?;

    Ok((user_id, identity_id))
}

async fn upsert_discord_profile(
    tx: &mut Transaction<'_, Postgres>,
    identity_id: Uuid,
    profile: &DiscordUserProfile,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        insert into discord_profiles (
            discord_identity_id, username, discriminator, global_name, avatar,
            bot, system, mfa_enabled, banner, accent_color, locale, verified,
            flags, premium_type, public_flags, avatar_decoration_data, collectibles, primary_guild
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        on conflict (discord_identity_id) do update set
            username = excluded.username,
            discriminator = excluded.discriminator,
            global_name = excluded.global_name,
            avatar = excluded.avatar,
            bot = excluded.bot,
            system = excluded.system,
            mfa_enabled = excluded.mfa_enabled,
            banner = excluded.banner,
            accent_color = excluded.accent_color,
            locale = excluded.locale,
            verified = excluded.verified,
            flags = excluded.flags,
            premium_type = excluded.premium_type,
            public_flags = excluded.public_flags,
            avatar_decoration_data = excluded.avatar_decoration_data,
            collectibles = excluded.collectibles,
            primary_guild = excluded.primary_guild
        "#,
    )
    .bind(identity_id)
    .bind(&profile.username)
    .bind(&profile.discriminator)
    .bind(&profile.global_name)
    .bind(&profile.avatar)
    .bind(profile.bot)
    .bind(profile.system)
    .bind(profile.mfa_enabled)
    .bind(&profile.banner)
    .bind(profile.accent_color)
    .bind(&profile.locale)
    .bind(profile.verified)
    .bind(profile.flags)
    .bind(profile.premium_type)
    .bind(profile.public_flags)
    .bind(&profile.avatar_decoration_data)
    .bind(&profile.collectibles)
    .bind(&profile.primary_guild)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn upsert_discord_tokens(
    tx: &mut Transaction<'_, Postgres>,
    identity_id: Uuid,
    tokens: &OAuthTokens,
    access_token_ciphertext: &str,
    refresh_token_ciphertext: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        insert into discord_oauth_tokens (
            discord_identity_id, access_token_ciphertext, refresh_token_ciphertext,
            token_type, scope, expires_at
        )
        values ($1, $2, $3, $4, $5, $6)
        on conflict (discord_identity_id) do update set
            access_token_ciphertext = excluded.access_token_ciphertext,
            refresh_token_ciphertext = excluded.refresh_token_ciphertext,
            token_type = excluded.token_type,
            scope = excluded.scope,
            expires_at = excluded.expires_at
        "#,
    )
    .bind(identity_id)
    .bind(access_token_ciphertext)
    .bind(refresh_token_ciphertext)
    .bind(&tokens.token_type)
    .bind(&tokens.scope)
    .bind(tokens.expires_at)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn load_auth_context(pool: &PgPool, token_hash: &str) -> Result<AuthContext, ApiError> {
    let context = sqlx::query_as::<_, AuthContext>(
        r#"
        select
            users.id as user_id,
            users.created_at as user_created_at,
            user_sessions.id as session_id,
            user_sessions.created_at as session_created_at,
            user_sessions.last_seen_at,
            user_sessions.expires_at,
            user_sessions.user_agent,
            user_sessions.login_location_country,
            user_sessions.login_location_region,
            user_sessions.login_location_city,
            user_sessions.current_location_country,
            user_sessions.current_location_region,
            user_sessions.current_location_city,
            discord_identities.discord_user_id,
            discord_profiles.username,
            discord_profiles.global_name,
            discord_profiles.avatar,
            discord_profiles.discriminator
        from user_sessions
        join users on users.id = user_sessions.user_id
        join discord_identities on discord_identities.user_id = users.id
        left join discord_profiles on discord_profiles.discord_identity_id = discord_identities.id
        where user_sessions.token_hash = $1
          and user_sessions.revoked_at is null
          and user_sessions.expires_at > now()
          and users.deleted_at is null
          and discord_identities.relogin_required_at is null
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    if let Some(context) = context {
        return Ok(context);
    }

    let status = sqlx::query_as::<_, SessionStatusRow>(
        r#"
        select user_sessions.revoked_at, user_sessions.expires_at,
               users.deleted_at, discord_identities.relogin_required_at
        from user_sessions
        join users on users.id = user_sessions.user_id
        join discord_identities on discord_identities.user_id = users.id
        where user_sessions.token_hash = $1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    match status {
        None => Err(ApiError::unauthorized("Authentication is required.")),
        Some(status) if status.revoked_at.is_some() => Err(ApiError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "auth.session_revoked",
            "Session has been revoked.",
        )),
        Some(status) if status.expires_at <= Utc::now() => Err(ApiError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "auth.session_expired",
            "Session has expired.",
        )),
        Some(status) if status.deleted_at.is_some() => {
            Err(ApiError::unauthorized("Authentication is required."))
        }
        Some(status) if status.relogin_required_at.is_some() => Err(ApiError::new(
            actix_web::http::StatusCode::UNAUTHORIZED,
            "auth.relogin_required",
            "Re-login is required.",
        )),
        Some(_) => Err(ApiError::unauthorized("Authentication is required.")),
    }
}

async fn validate_csrf_async(
    state: &AppState,
    req: &HttpRequest,
    session_id: Uuid,
) -> Result<(), ApiError> {
    let csrf_header = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::forbidden("auth.csrf_required", "CSRF token is required."))?;

    let expected = csrf_hash_for_session(&state.db_pool, session_id).await?;
    let candidate = crypto::token_hash(
        &state.config.security.session_hmac_secret,
        HashDomain::Csrf,
        csrf_header,
    );

    if !crypto::constant_time_eq(&expected, &candidate) {
        write_audit_event(
            &state.db_pool,
            None,
            Some(session_id),
            "csrf_validation_failed",
            request_ip(req).map(|ip| ip.to_string()).as_deref(),
            request_user_agent(req).as_deref(),
            json!({}),
        )
        .await?;
        return Err(ApiError::forbidden(
            "auth.csrf_invalid",
            "CSRF token is invalid.",
        ));
    }

    Ok(())
}

async fn write_audit_event(
    pool: &PgPool,
    user_id: Option<Uuid>,
    session_id: Option<Uuid>,
    event_type: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    metadata: Value,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        insert into security_audit_events
            (user_id, session_id, event_type, ip_address, user_agent, metadata)
        values ($1, $2, $3, $4::inet, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(session_id)
    .bind(event_type)
    .bind(ip)
    .bind(user_agent)
    .bind(metadata)
    .execute(pool)
    .await?;

    Ok(())
}

async fn write_audit_event_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Option<Uuid>,
    session_id: Option<Uuid>,
    event_type: &str,
    ip: Option<&str>,
    user_agent: Option<&str>,
    metadata: Value,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        insert into security_audit_events
            (user_id, session_id, event_type, ip_address, user_agent, metadata)
        values ($1, $2, $3, $4::inet, $5, $6)
        "#,
    )
    .bind(user_id)
    .bind(session_id)
    .bind(event_type)
    .bind(ip)
    .bind(user_agent)
    .bind(metadata)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

fn safe_user(context: &AuthContext) -> SafeUserDto {
    SafeUserDto {
        id: context.user_id,
        created_at: context.user_created_at,
        discord: SafeDiscordUserDto {
            id: context.discord_user_id.clone(),
            username: context.username.clone(),
            global_name: context.global_name.clone(),
            avatar_url: avatar_url(
                &context.discord_user_id,
                context.avatar.as_deref(),
                context.discriminator.as_deref(),
            ),
        },
    }
}

fn safe_session_from_context(context: &AuthContext, is_current: bool) -> SafeSessionDto {
    SafeSessionDto {
        id: context.session_id,
        created_at: context.session_created_at,
        last_seen_at: context.last_seen_at,
        expires_at: context.expires_at,
        user_agent: context.user_agent.clone(),
        login_location: LocationDto {
            country: context.login_location_country.clone(),
            region: context.login_location_region.clone(),
            city: context.login_location_city.clone(),
        },
        current_location: LocationDto {
            country: context.current_location_country.clone(),
            region: context.current_location_region.clone(),
            city: context.current_location_city.clone(),
        },
        is_current,
    }
}

fn safe_session_from_row(row: SessionRow, current_session_id: Uuid) -> SafeSessionDto {
    SafeSessionDto {
        id: row.id,
        created_at: row.created_at,
        last_seen_at: row.last_seen_at,
        expires_at: row.expires_at,
        user_agent: row.user_agent,
        login_location: LocationDto {
            country: row.login_location_country,
            region: row.login_location_region,
            city: row.login_location_city,
        },
        current_location: LocationDto {
            country: row.current_location_country,
            region: row.current_location_region,
            city: row.current_location_city,
        },
        is_current: row.id == current_session_id,
    }
}

fn session_cookie(config: &CookieConfig, value: &str, remove: bool) -> Cookie<'static> {
    build_cookie(config, &config.session_name, value, true, remove)
}

fn csrf_cookie(config: &CookieConfig, value: &str, remove: bool) -> Cookie<'static> {
    build_cookie(config, &config.csrf_name, value, false, remove)
}

fn build_cookie(
    config: &CookieConfig,
    name: &str,
    value: &str,
    http_only: bool,
    remove: bool,
) -> Cookie<'static> {
    let max_age = if remove {
        CookieDuration::seconds(0)
    } else {
        CookieDuration::days(SESSION_TTL_DAYS)
    };

    let mut builder = Cookie::build(name.to_string(), value.to_string())
        .path("/")
        .http_only(http_only)
        .secure(config.secure)
        .same_site(config.same_site)
        .max_age(max_age);

    if config.domain != "localhost" {
        builder = builder.domain(config.domain.clone());
    }

    builder.finish()
}

fn request_user_agent(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn request_ip(req: &HttpRequest) -> Option<IpAddr> {
    req.connection_info()
        .realip_remote_addr()
        .and_then(parse_ip)
}

fn parse_ip(value: &str) -> Option<IpAddr> {
    value
        .parse()
        .ok()
        .or_else(|| value.rsplit_once(':').and_then(|(ip, _)| ip.parse().ok()))
}
