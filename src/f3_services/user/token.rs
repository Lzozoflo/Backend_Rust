//! JWT creation and verification (HS256).
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (User UUID).
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

pub fn create_token(user_id: Uuid, secret: &str, ttl_secs: u64) -> Result<String, AppError> {
    let iat = now();
    let claims = Claims { sub: user_id.to_string(), iat, exp: iat + ttl_secs };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::Internal(format!("jwt: {e}")))
}

/// Returns the user's `Uuid` if the token signature is valid and not expired.
pub fn verify_token(token: &str, secret: &str) -> Result<Uuid, AppError> {
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map_err(|_| AppError::Unauthorized)?;
    Uuid::parse_str(&data.claims.sub).map_err(|_| AppError::Unauthorized)
}

// ═══════════════════════════════ UNIT TEST ═══════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_roundtrip() {
        let id = Uuid::new_v4();
        let token = create_token(id, "secret", 60).unwrap();
        assert_eq!(verify_token(&token, "secret").unwrap(), id);
    }

    #[test]
    fn wrong_secret_is_rejected() {
        let token = create_token(Uuid::new_v4(), "secret", 60).unwrap();
        assert!(verify_token(&token, "other").is_err());
    }

    #[test]
    fn expired_token_is_rejected() {
        let claims = Claims { sub: Uuid::new_v4().to_string(), iat: 1, exp: 2 };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"secret")).unwrap();
        assert!(verify_token(&token, "secret").is_err());
    }

    #[test]
    fn non_uuid_subject_is_rejected() {
        let claims = Claims { sub: "not-a-uuid".into(), iat: now(), exp: now() + 60 };
        let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(b"secret")).unwrap();
        assert!(verify_token(&token, "secret").is_err());
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(verify_token("garbage.token.value", "secret").is_err());
    }
}
