# Raid Composition Backend

Backend API for a raid composition application. The service is written in Rust with Actix Web, uses PostgreSQL through SQLx, uses Redis for cache/session-ready infrastructure, and implements Discord OAuth authentication with server-side sessions.

## Stack

- Rust 2024 edition
- Actix Web
- SQLx `0.8.6` with PostgreSQL
- Redis with the `redis` Rust crate
- Discord OAuth with HTTP-only server-side sessions
- Docker and Docker Compose for local development
- GitHub Actions for clippy, tests, and Docker image builds

## Project Structure

```text
src/
  main.rs                 # Actix server bootstrap
  config.rs               # Environment-driven application config
  db.rs                   # PostgreSQL connection setup
  auth/                   # Auth service, Discord client, crypto, and GeoIP helpers
  api/
    routes/               # Versioned route registration
      v1/
        auth/             # Authentication route modules
        health/           # App, PostgreSQL, and Redis health route modules
    error.rs              # Structured JSON API errors
migrations/
  *.sql                   # Embedded SQLx migrations
Dockerfile                # Production image build
local.Dockerfile          # Compose development image with cargo-watch and sqlx-cli
docker-compose.yml        # Local PostgreSQL, Redis, and API services
```

## Configuration

The application reads required configuration from environment variables. During local development, `dotenv` loads values from `.env` once during startup.

Create your local environment file from the example:

```bash
cp .env.example .env
```

Required variables:

| Variable | Description |
| --- | --- |
| `APP_PORT` | Port the Actix server binds to. Use `8000` with the current Docker Compose setup. |
| `FRONTEND_BASE_URL` | Frontend base URL used for credentialed CORS origin checks. |
| `DB_HOST` | PostgreSQL host. Use `localhost` for native local runs or `postgres` inside Docker Compose. |
| `DB_PORT` | PostgreSQL port. |
| `DB_USER` | PostgreSQL user. |
| `DB_PASSWORD` | PostgreSQL password. |
| `DB_NAME` | PostgreSQL database name. |
| `REDIS_HOST` | Redis host. Use `localhost` for native local runs or `redis` inside Docker Compose. The Compose API container overrides this to `redis`. |
| `REDIS_PORT` | Redis port. |
| `REDIS_PASSWORD` | Redis password. Redis starts with `--requirepass`; Docker Compose defaults this to `password` if unset. |
| `DISCORD_CLIENT_ID` | Discord OAuth application client ID. |
| `DISCORD_CLIENT_SECRET` | Discord OAuth application client secret. |
| `DISCORD_REDIRECT_URL` | Exact Discord OAuth redirect URL sent during authorization and token exchange. |
| `DISCORD_TOKEN_ENCRYPTION_KEY` | 32-byte Discord token encryption key, encoded as hex, standard base64, or unpadded URL-safe base64. |
| `SESSION_HMAC_SECRET` | Secret used to HMAC session, CSRF, and OAuth state tokens. Must be at least 32 bytes. |
| `GEOIP_DATABASE_PATH` | Path to a MaxMind GeoLite2 City `.mmdb` file. Missing files disable GeoIP lookup without failing startup. |
| `COOKIE_DOMAIN` | Cookie domain for auth/session cookies. |

Optional cookie variables:

| Variable | Default | Description |
| --- | --- | --- |
| `COOKIE_SECURE` | `true` | Whether auth cookies require HTTPS. Use `false` for local plain HTTP. |
| `COOKIE_SAME_SITE` | `Lax` | Cookie SameSite policy: `Lax`, `Strict`, or `None`. |
| `SESSION_COOKIE_NAME` | `session` | HTTP-only session cookie name. |
| `CSRF_COOKIE_NAME` | `csrf` | Readable CSRF cookie name. |

The API requires every required variable above to be present, non-empty, and valid at startup. Ports must be non-zero `u16` values. Missing, malformed, or weak auth configuration stops the server before it binds an HTTP port.

The application runtime uses the `DB_*` variables above and does not require `DATABASE_URL`. `DATABASE_URL` is only needed when running manual SQLx CLI commands.

Example Docker Compose-oriented values:

```env
APP_PORT=8000
FRONTEND_BASE_URL=http://localhost:4200

DB_HOST=postgres
DB_PORT=5432
DB_USER=user
DB_PASSWORD=password
DB_NAME=app_db

REDIS_HOST=redis
REDIS_PORT=6379
REDIS_PASSWORD=password

DISCORD_CLIENT_ID=your_discord_client_id
DISCORD_CLIENT_SECRET=your_discord_client_secret
DISCORD_REDIRECT_URL=http://localhost:4200/auth/discord/callback
DISCORD_TOKEN_ENCRYPTION_KEY=0000000000000000000000000000000000000000000000000000000000000000
SESSION_HMAC_SECRET=replace-with-at-least-32-random-bytes
GEOIP_DATABASE_PATH=/app/local/GeoLite2-City.mmdb

COOKIE_DOMAIN=localhost
COOKIE_SECURE=false
COOKIE_SAME_SITE=Lax
```

Generate local development secrets with:

```bash
openssl rand -hex 32
```

Use a different generated value for `DISCORD_TOKEN_ENCRYPTION_KEY` and `SESSION_HMAC_SECRET`.

## Running Locally

Start PostgreSQL and Redis with Docker Compose:

```bash
docker compose up postgres redis
```

Use `DB_HOST=localhost` and `REDIS_HOST=localhost` in `.env` when running the API directly on the host. Keep `REDIS_PASSWORD` aligned with the password used by the Redis container, then start the server:

```bash
cargo run
```

The API connects to PostgreSQL, runs embedded SQLx migrations, creates the Redis client, then binds to `0.0.0.0:${APP_PORT}`. If migrations fail, startup stops before the HTTP port is bound.

GeoIP lookup is best-effort. To enable it locally, download a MaxMind GeoLite2 City database and set `GEOIP_DATABASE_PATH` to the `.mmdb` file path visible to the API process. If the file is missing, GeoIP is disabled and location fields remain `null`.

## Running With Docker Compose

Run the API, PostgreSQL, and Redis together:

```bash
docker compose up --build
```

The local Docker image uses `cargo-watch`. Changes under `src/` are synced into the container. Dependency or manifest changes in `Cargo.toml` require rebuilding/recreating the API container so the container sees the updated manifest and lockfile:

```bash
docker compose up -d --build api
```

Environment changes in `.env` require recreating the container, not just restarting it:

```bash
docker compose up -d --force-recreate api
```

With the current `docker-compose.yml`, keep `APP_PORT=8000` in `.env` because the API service exposes container port `8000`. Redis is exposed on `${REDIS_PORT:-6379}` and requires `REDIS_PASSWORD` for clients. Compose still uses shell defaults for local infrastructure convenience, but the API runtime itself requires explicit environment values.

`docker-compose.yml` builds the API from `local.Dockerfile`. The local image includes `cargo-watch` for the development loop and `sqlx-cli` for migration commands. The production `Dockerfile` is separate and builds a locked release binary for a minimal runtime image.

## Database Migrations

SQLx migrations are embedded into the application binary and run automatically on every API startup. Startup order is PostgreSQL pool creation, migration execution, Redis client creation, then HTTP server bind. If migration execution fails, startup stops with `Failed to run database migrations` and the HTTP server does not bind.

Migration files live in `migrations/`. Existing migrations may be single `.sql` files. For new schema changes, prefer reversible `.up.sql` and `.down.sql` pairs:

```text
migrations/
  20260430120000_create_some_table.up.sql
  20260430120000_create_some_table.down.sql
```

Do not edit migrations after they have been applied in a shared environment. SQLx records checksums, so changing applied files can create version or checksum conflicts. Treat a migration as irreversible if it destroys data, transforms data non-bijectively, depends on external state, or would require guessing to revert. Irreversible migrations must include a `.down.sql` file that fails explicitly with a clear reason.

Migration file changes are not watched by Docker Compose. Restart or rebuild the API container after adding or changing migrations so the embedded migration set is compiled into the binary.

### Docker Compose SQLx CLI

The local API image includes `sqlx-cli` pinned to the same SQLx version used by the application. Use the Docker network host name `postgres` for CLI commands run through Compose:

```bash
export DATABASE_URL=postgres://user:password@postgres:5432/app_db

docker compose run --rm -e DATABASE_URL api sqlx migrate add -r create_some_table
docker compose run --rm -e DATABASE_URL api sqlx migrate run
docker compose run --rm -e DATABASE_URL api sqlx migrate revert
```

The `-r` flag creates the required reversible `.up.sql` and `.down.sql` files. `DATABASE_URL` is passed to the one-off container for SQLx CLI use only; the application runtime still reads `DB_HOST`, `DB_PORT`, `DB_USER`, `DB_PASSWORD`, and `DB_NAME`.

### Host SQLx CLI

Host CLI usage is optional. Install the matching CLI version with PostgreSQL support:

```bash
cargo install sqlx-cli --version 0.8.6 --no-default-features --features postgres
```

Use `localhost` for the database host when running SQLx from the host against the Compose PostgreSQL port mapping:

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/app_db

sqlx migrate add -r create_some_table
sqlx migrate run
sqlx migrate revert
```

## API Endpoints

Base path: `/api/v1`

| Method | Path | Status |
| --- | --- | --- |
| `GET` | `/auth/discord/url` | Creates a server-side OAuth state and returns a Discord OAuth authorization URL. |
| `POST` | `/auth/discord/callback` | Exchanges Discord code, stores identity/profile/encrypted tokens, creates session and CSRF cookies, and returns `204 No Content`. |
| `GET` | `/auth/session` | Returns the current authenticated user and session. |
| `GET` | `/auth/sessions` | Lists active sessions for the current user. |
| `POST` | `/auth/logout` | Revokes the current session and clears auth cookies. |
| `POST` | `/auth/logout-all-other-sessions` | Revokes all active sessions except the current session. |
| `DELETE` | `/auth/sessions/{session_id}` | Revokes another active session owned by the current user. |
| `GET` | `/auth/csrf` | Refreshes the readable CSRF cookie for the current session. |
| `GET` | `/health` | Checks application liveness. |
| `GET` | `/health/postgres` | Checks PostgreSQL connectivity with `SELECT 1`. |
| `GET` | `/health/redis` | Checks Redis connectivity with an authenticated `PING`. |

Example:

```bash
curl http://localhost:8000/api/v1/auth/discord/url
curl http://localhost:8000/api/v1/health
curl http://localhost:8000/api/v1/health/postgres
curl http://localhost:8000/api/v1/health/redis
```

Authenticated mutating requests use cookie authentication and require the readable CSRF cookie value in the `X-CSRF-Token` header. Frontend requests must include credentials:

```ts
fetch("http://localhost:8000/api/v1/auth/logout", {
  method: "POST",
  credentials: "include",
  headers: { "X-CSRF-Token": csrfToken },
});
```

## Development Commands

```bash
cargo check
cargo test --workspace --locked --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
docker compose build api
```

## Docker Images

`Dockerfile` builds a locked release binary in a Rust Alpine builder image and copies it into a `scratch` runtime image.

The production runtime image does not include `sqlx-cli` or migration files on disk. Migrations are embedded during the builder stage while `migrations/` is present in the build context, so the final image can remain `FROM scratch`.

Production also runs migrations on startup from the application binary. For future multi-replica Kubernetes deployments, this may move to a dedicated pre-deploy step or Kubernetes Job before application pods roll out.

The GitHub Actions workflow runs on pushes and pull requests to `master`, plus published GitHub releases. It performs clippy and tests, builds the production Docker image for `linux/amd64`, and publishes to GHCR and Docker Hub when publishing is allowed:

- Pushes to `master` publish `latest`, `edge-master`, and `sha-*` tags.
- Published releases require a `vMAJOR.MINOR.PATCH` or `MAJOR.MINOR.PATCH` tag and publish semver tags.
- Pull requests from the same repository publish temporary `pr-*` tags.
- Closed same-repository pull requests trigger cleanup of the temporary PR image tags.

The Docker build enables provenance and SBOM output. Docker Scout reports high vulnerabilities and fails the workflow on critical vulnerabilities for published images.

## Current Notes

- Database and Redis dependencies are initialized during startup and injected into routes through shared application state.
- Health endpoints use the shared PostgreSQL pool and Redis client.
- Auth sessions are durable in PostgreSQL. Redis is available in application state for future session caching but is not used as the session source of truth yet.
- Discord OAuth requests the `identify` scope only. Email login, passwords, roles, and permissions are not implemented.
- GeoIP lookup is optional. Missing local MaxMind databases leave session location fields as `null`.
