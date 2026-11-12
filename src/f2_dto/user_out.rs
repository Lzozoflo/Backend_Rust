//! DTO de sortie de l'entité user : jamais de hash de mot de passe.
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::f4_repo::user::User;

/// UserResponse
/// when a user was created
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self { id: user.id, username: user.username, email: user.email, created_at: user.created_at }
    }
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub user: UserResponse,
}
