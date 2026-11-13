//! Database connection pool (diesel-async + bb8) and embedded migrations.
use std::time::Duration;

use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{bb8::Pool, AsyncDieselConnectionManager},
    AsyncConnection, AsyncPgConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub type Db = Pool<AsyncPgConnection>;
pub type MigrationError = Box<dyn std::error::Error + Send + Sync>;

/// Migrations SQL de `migrations/`, embarquées dans le binaire à la compilation.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

const MAX_CONNECTIONS: u32 = 10;
const CONNECT_ATTEMPTS: u32 = 10;
const RETRY_DELAY: Duration = Duration::from_secs(2);

fn manager(url: &str) -> AsyncDieselConnectionManager<AsyncPgConnection> {
    AsyncDieselConnectionManager::new(url)
}

/// Creates a pool that only connects on the first query (useful for database-less tests).
pub fn connect_lazy(url: &str) -> Db {
    Db::builder().max_size(MAX_CONNECTIONS).build_unchecked(manager(url))
}

/// Connects to PostgreSQL with automatic retries, handling cases where the database starts after the backend.
pub async fn connect(url: &str) -> Result<Db, MigrationError> {
    let mut attempt = 1;
    loop {
        match Db::builder().max_size(MAX_CONNECTIONS).build(manager(url)).await {
            Ok(pool) => return Ok(pool),
            Err(err) if attempt < CONNECT_ATTEMPTS => {
                tracing::warn!(attempt, max = CONNECT_ATTEMPTS, %err, "database unreachable, retrying");
                attempt += 1;
                tokio::time::sleep(RETRY_DELAY).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
}

/// Runs pending SQL migrations from `migrations/`.
/// `diesel_migrations` is synchronous: the async connection is wrapped and run on a blocking thread.
pub async fn migrate(url: &str) -> Result<(), MigrationError> {
    let connection = AsyncPgConnection::establish(url).await?;
    let mut wrapper: AsyncConnectionWrapper<AsyncPgConnection> = AsyncConnectionWrapper::from(connection);
    tokio::task::spawn_blocking(move || -> Result<(), MigrationError> {
        wrapper.run_pending_migrations(MIGRATIONS)?;
        Ok(())
    })
    .await??;
    Ok(())
}
