use sha2::{Digest, Sha256};
use sqlx::{PgPool, migrate::MigrateError};

use crate::config::DatabaseConfig;

const SEED_HISTORY_LOCK_KEY: i64 = 0x5241_4944_5345_4544;

struct EmbeddedSeed {
    name: &'static str,
    up_sql: &'static str,
}

const EMBEDDED_SEEDS: &[EmbeddedSeed] = &[EmbeddedSeed {
    name: "20260504_character_roster",
    up_sql: include_str!("../seeds/20260504_character_roster.up.sql"),
}];

pub async fn connect(config: &DatabaseConfig) -> Result<PgPool, sqlx::Error> {
    let pool = PgPool::connect(&config.url()).await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn run_seeds(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;

    sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(SEED_HISTORY_LOCK_KEY)
        .execute(&mut *transaction)
        .await?;

    sqlx::query(
        r#"
        create table if not exists seed_history (
            name text primary key,
            checksum text not null,
            applied_at timestamptz not null default now()
        )
        "#,
    )
    .execute(&mut *transaction)
    .await?;

    for seed in EMBEDDED_SEEDS {
        let checksum = seed_checksum(seed.up_sql);
        let existing_checksum = sqlx::query_scalar::<_, String>(
            r#"
            select checksum
            from seed_history
            where name = $1
            "#,
        )
        .bind(seed.name)
        .fetch_optional(&mut *transaction)
        .await?;

        match existing_checksum {
            Some(existing_checksum) if existing_checksum == checksum => continue,
            Some(_) => {
                return Err(sqlx::Error::Protocol(format!(
                    "seed {} was already applied with a different checksum",
                    seed.name
                )));
            }
            None => {}
        }

        sqlx::raw_sql(seed.up_sql)
            .execute(&mut *transaction)
            .await?;

        sqlx::query(
            r#"
            insert into seed_history (name, checksum)
            values ($1, $2)
            "#,
        )
        .bind(seed.name)
        .bind(checksum)
        .execute(&mut *transaction)
        .await?;
    }

    transaction.commit().await
}

fn seed_checksum(sql: &str) -> String {
    let digest = Sha256::digest(sql.as_bytes());
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
