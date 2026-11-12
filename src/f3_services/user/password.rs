//! Hachage Argon2id avec sel aléatoire. Le hachage est coûteux en CPU :
//! les versions `async` l'exécutent hors du runtime via `spawn_blocking`.
use std::sync::OnceLock;

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};

use crate::error::AppError;

/// Hache un mot de passe (le sel est généré et inclus dans le hash PHC).
pub fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::Internal(format!("hachage : {e}")))
}


/// Compare un mot de passe à un hash PHC.
///
/// Un hash illisible ne valide jamais (retourne `false`).
///
/// # Exemple
///
/// ```
/// use backend_rust::f3_services::user::password::{hash_password, verify_password};
///
/// let hash = hash_password("motdepasse1").unwrap();
/// assert!(verify_password("motdepasse1", &hash));
/// assert!(!verify_password("autre", &hash));
/// ```
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

pub async fn hash(password: String) -> Result<String, AppError> {
    run_blocking(move || hash_password(&password)).await?
}

pub async fn verify(password: String, hash: String) -> Result<bool, AppError> {
    run_blocking(move || verify_password(&password, &hash)).await
}

/// Fait payer le coût d'une vérification même quand l'email est inconnu, pour que
/// le temps de réponse ne révèle pas si un compte existe.
pub async fn verify_dummy(password: String) -> Result<(), AppError> {
    static DUMMY_HASH: OnceLock<String> = OnceLock::new();
    run_blocking(move || {
        let hash = DUMMY_HASH.get_or_init(|| hash_password("mot-de-passe-factice-1").unwrap_or_default());
        verify_password(&password, hash);
    })
    .await
}

async fn run_blocking<T, F>(job: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(job)
        .await
        .map_err(|e| AppError::Internal(format!("tâche bloquante : {e}")))
}

// ═══════════════════════════════ UNIT TEST ═══════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_succeeds() {
        let hash = hash_password("Sup3rSecret!").unwrap();
        assert!(verify_password("Sup3rSecret!", &hash));
    }

    #[test]
    fn wrong_password_is_rejected() {
        let hash = hash_password("Sup3rSecret!").unwrap();
        assert!(!verify_password("autre-mot-de-passe1", &hash));
    }

    #[test]
    fn hash_is_not_plaintext_and_is_argon2id() {
        let hash = hash_password("Sup3rSecret!").unwrap();
        assert!(!hash.contains("Sup3rSecret!"));
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn same_password_gives_different_hashes_thanks_to_salt() {
        let a = hash_password("Sup3rSecret!").unwrap();
        let b = hash_password("Sup3rSecret!").unwrap();
        assert_ne!(a, b);
        assert!(verify_password("Sup3rSecret!", &a) && verify_password("Sup3rSecret!", &b));
    }

    #[test]
    fn malformed_hash_never_verifies() {
        assert!(!verify_password("x", "pas-un-hash"));
        assert!(!verify_password("x", ""));
    }

    #[tokio::test]
    async fn async_wrappers_roundtrip() {
        let hash = hash("Sup3rSecret!".into()).await.unwrap();
        assert!(verify("Sup3rSecret!".into(), hash.clone()).await.unwrap());
        assert!(!verify("mauvais".into(), hash).await.unwrap());
    }

    #[tokio::test]
    async fn dummy_verification_never_fails() {
        verify_dummy("n-importe-quoi".into()).await.unwrap();
    }
}
