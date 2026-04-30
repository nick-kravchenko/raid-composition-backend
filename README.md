# Raid Composition Backend

Backend API for a raid composition application. The service is written in Rust with Actix Web, uses PostgreSQL through SQLx, uses Redis for cache/session-ready infrastructure, and currently contains the first authentication route scaffolding for Discord OAuth.

## Stack

- Rust 2024 edition
- Actix Web
- SQLx with PostgreSQL
- Redis with the `redis` Rust crate
- Docker and Docker Compose for local development
- GitHub Actions for clippy, tests, and Docker image builds

## Project Structure

```text
src/
  main.rs                 # Actix server bootstrap
  config.rs               # Environment-driven application config
  db.rs                   # PostgreSQL connection setup
  api/
    routes/               # Versioned route registration
      v1/
        auth/             # Authentication route modules
        health/           # App, PostgreSQL, and Redis health route modules
    controllers/          # Request handling helpers
    dto/                  # API/data transfer structs
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
| `FRONTEND_BASE_URL` | Frontend base URL used exactly as the Discord OAuth redirect URI. |
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
| `COOKIE_DOMAIN` | Cookie domain for auth/session cookies. |

The API requires every variable above to be present, non-empty, and valid at startup. Ports must be non-zero `u16` values. Missing or invalid configuration stops the server before it binds an HTTP port.

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

COOKIE_DOMAIN=localhost
```

## Running Locally

Start PostgreSQL and Redis with Docker Compose:

```bash
docker compose up postgres redis
```

Use `DB_HOST=localhost` and `REDIS_HOST=localhost` in `.env` when running the API directly on the host. Keep `REDIS_PASSWORD` aligned with the password used by the Redis container, then start the server:

```bash
cargo run
```

The server binds to `0.0.0.0:${APP_PORT}`.

## Running With Docker Compose

Run the API, PostgreSQL, and Redis together:

```bash
docker compose up --build
```

The local Docker image uses `cargo-watch`, so changes under `src/` are synced into the container and the application is rebuilt when `Cargo.toml` changes.

With the current `docker-compose.yml`, keep `APP_PORT=8000` in `.env` because the API service exposes container port `8000`. Redis is exposed on `${REDIS_PORT:-6379}` and requires `REDIS_PASSWORD` for clients. Compose still uses shell defaults for local infrastructure convenience, but the API runtime itself requires explicit environment values.

## API Endpoints

Base path: `/api/v1`

| Method | Path | Status |
| --- | --- | --- |
| `GET` | `/auth/discord/url` | Returns a Discord OAuth authorization URL. |
| `GET` | `/auth/me` | Placeholder user info response. |
| `POST` | `/auth/logout` | Returns `204 No Content`. |
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

## Development Commands

```bash
cargo check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

## Docker Images

`Dockerfile` builds a release binary in a Rust Alpine builder image and copies it into a `scratch` runtime image.

The GitHub Actions workflow runs on pushes and pull requests to `master`. It performs Rust checks/tests, then builds and publishes Docker images to GHCR and Docker Hub for non-PR events.

## Current Notes

- Database and Redis dependencies are initialized during startup and injected into routes through shared application state.
- Health endpoints use the shared PostgreSQL pool and Redis client.
- There are no migrations in the repository yet.
- Discord OAuth is only partially implemented. The authorization URL endpoint exists, but callback handling and token exchange are not implemented.
- Some auth/session endpoints currently return placeholder responses.
