//! SQL queries for the `user` entity. Contains no business logic.
//!
//! User-supplied values ALWAYS go through `.bind()` (parameterized queries `$1, $2…`):
//! string concatenation is never used, preventing SQL injection.

use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::Db;
use crate::error::AppError;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
}

/// Partial update payload: only `Some` fields are modified.
#[derive(Debug, Default)]
pub struct UserChanges<'a> {
    pub username: Option<&'a str>,
    pub email: Option<&'a str>,
    pub password_hash: Option<&'a str>,
}

pub async fn insert(db: &Db, new: NewUser<'_>) -> Result<User, AppError> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, username, email, password_hash)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(Uuid::now_v7())
    .bind(new.username)
    .bind(new.email)
    .bind(new.password_hash)
    .fetch_one(db)
    .await?;
    Ok(user)
}

pub async fn find_by_email(db: &Db, email: &str) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(db)
        .await?;
    Ok(user)
}

pub async fn update(db: &Db, id: Uuid, changes: UserChanges<'_>) -> Result<Option<User>, AppError> {
    let user = sqlx::query_as::<_, User>(
        "UPDATE users
         SET username      = COALESCE($2, username),
             email         = COALESCE($3, email),
             password_hash = COALESCE($4, password_hash),
             updated_at    = now()
         WHERE id = $1
         RETURNING *",
    )
    .bind(id)
    .bind(changes.username)
    .bind(changes.email)
    .bind(changes.password_hash)
    .fetch_optional(db)
    .await?;
    Ok(user)
}

/// Deletes a user by ID. Returns `true` if a row was deleted.
pub async fn delete(db: &Db, id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(db).await?;
    Ok(result.rows_affected() > 0)
}
