//! Pool de connexions et migrations.
use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, Connection, PgConnection, PgPool};

pub type Db = PgPool;
pub type MigrationError = Box<dyn std::error::Error + Send + Sync>;

const MAX_CONNECTIONS: u32 = 10;
const CONNECT_ATTEMPTS: u32 = 10;
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// Pool qui ne se connecte qu'à la première requête (utile pour les tests sans base).
pub fn connect_lazy(url: &str) -> Db {
    PgPoolOptions::new().max_connections(MAX_CONNECTIONS).connect_lazy(url).expect("DATABASE_URL invalide")
}

/// Pool connecté, avec quelques tentatives : la base peut démarrer après le backend.
pub async fn connect(url: &str) -> Result<Db, sqlx::Error> {
    let mut attempt = 1;
    loop {
        match PgPoolOptions::new().max_connections(MAX_CONNECTIONS).connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(err) if attempt < CONNECT_ATTEMPTS => {
                tracing::warn!(attempt, max = CONNECT_ATTEMPTS, %err, "base injoignable, nouvelle tentative");
                attempt += 1;
                tokio::time::sleep(RETRY_DELAY).await;
            }
            Err(err) => return Err(err),
        }
    }
}

/// Applique les migrations du dossier `migrations/` (embarquées dans le binaire).
pub async fn migrate(url: &str) -> Result<(), MigrationError> {
    let mut connection = PgConnection::connect(url).await?;
    sqlx::migrate!("./migrations").run(&mut connection).await?;
    Ok(())
}
