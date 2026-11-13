//! Application configuration, loaded from environment variables.
use std::{env, fmt, str::FromStr};

// Secret JWT de développement. Refusé en production par `docker-compose.prod.yml`.
pub const DEV_TOKEN_SECRET: &str = "dev-only-secret-change-me";

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub app_version: String,
    pub token_secret: String,
    /// Durée de vie d'un token, en secondes.
    pub token_expiration: u64,
}

impl Config {
    /// Reads configuration from the environment. Missing variables fall back to default values;
    /// present but invalid variables cause startup to panic (fail-fast).
    pub fn from_env() -> Self {
        Self {
            database_url: var_or("DATABASE_URL", "postgres://postgres:postgres@localhost:5432/backend_rust_db".to_string()),
            port: var_or("PORT", 8080),
            app_version: var_or("APP_VERSION", "0.1".to_string()),
            token_secret: var_or("TOKEN_SECRET", DEV_TOKEN_SECRET.to_string()),
            token_expiration: var_or("TOKEN_EXPIRATION", 3600),
        }
    }

    pub fn uses_dev_secret(&self) -> bool {
        self.token_secret == DEV_TOKEN_SECRET
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("app_version", &self.app_version)
            .field("token_expiration", &self.token_expiration)
            .finish_non_exhaustive()
    }
}

fn var_or<T>(key: &str, default: T) -> T
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match env::var(key) {
        Ok(raw) => raw.parse().unwrap_or_else(|e| panic!("invalid variable {key}={raw:?}: {e}")),
        Err(_) => default,
    }
}

// ═══════════════════════════════ UNIT TEST ═══════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    fn config(secret: &str) -> Config {
        Config {
            database_url: "postgres://secret-user:secret-pass@db/x".into(),
            port: 1,
            app_version: "t".into(),
            token_secret: secret.into(),
            token_expiration: 1,
        }
    }

    #[test]
    fn debug_never_leaks_secrets() {
        let out = format!("{:?}", config("very-secret"));
        assert!(!out.contains("very-secret"));
        assert!(!out.contains("secret-pass"));
    }

    #[test]
    fn detects_dev_secret() {
        assert!(config(DEV_TOKEN_SECRET).uses_dev_secret());
        assert!(!config("other-secret").uses_dev_secret());
    }

    #[test]
    fn var_or_uses_default_when_unset() {
        assert_eq!(var_or("TSG_NON_EXISTENT_VARIABLE", 42u16), 42);
    }
}
