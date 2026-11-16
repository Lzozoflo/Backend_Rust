//! Diesel queries for the `user` entity. Contains no business logic.
//!
//! Diesel's query builder always sends user-supplied values as bind parameters:
//! string concatenation is never used, preventing SQL injection.

use chrono::{DateTime, Utc};
use diesel::{dsl::now, prelude::*};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use super::{error::pool_error, Db};
use crate::{error::AppError, schema::users};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub token_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


pub async fn get_token_version(db: &Db, id: Uuid) -> Result<Option<i32>, AppError> {
    let mut conn = db.get().await.map_err(pool_error)?;
    users::table
        .find(id)
        .select(users::token_version)
        .first(&mut conn)
        .await
        .optional()
        .map_err(Into::into)
}


/// Incrémente atomiquement token_version pour invalider tous les JWT existants.
pub async fn increment_token_version(db: &Db, id: Uuid) -> Result<Option<i32>, AppError> {
    let mut conn = db.get().await.map_err(pool_error)?;
    diesel::update(users::table.find(id))
        .set((
            users::token_version.eq(users::token_version + 1),
            users::updated_at.eq(now),
        ))
        .returning(users::token_version)
        .get_result(&mut conn)
        .await
        .optional()
        .map_err(Into::into)
}

pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
}

pub async fn insert(db: &Db, new: NewUser<'_>) -> Result<User, AppError> {
    let mut conn = db.get().await.map_err(pool_error)?;
    let user = diesel::insert_into(users::table)
        .values((
            users::id.eq(Uuid::now_v7()),
            users::token_version.eq(1),
            users::username.eq(new.username),
            users::email.eq(new.email),
            users::password_hash.eq(new.password_hash),
        ))
        .returning(User::as_returning())
        .get_result(&mut conn)
        .await?;
    Ok(user)
}

pub async fn find_by_email(db: &Db, email: &str) -> Result<Option<User>, AppError> {
    let mut conn = db.get().await.map_err(pool_error)?;
    let user = users::table
        .filter(users::email.eq(email))
        .select(User::as_select())
        .first(&mut conn)
        .await
        .optional()?;
    Ok(user)
}

/// Partial update payload: only `Some` fields are modified (`None` fields are skipped by Diesel).
#[derive(Debug, Default, AsChangeset)]
#[diesel(table_name = users)]
pub struct UserChanges<'a> {
    pub username: Option<&'a str>,
    pub email: Option<&'a str>,
    pub password_hash: Option<&'a str>,
}

pub async fn update(db: &Db, id: Uuid, changes: UserChanges<'_>) -> Result<Option<User>, AppError> {
    let mut conn = db.get().await.map_err(pool_error)?;
    let user = diesel::update(users::table.find(id))
        .set((changes, users::updated_at.eq(now)))
        .returning(User::as_returning())
        .get_result(&mut conn)
        .await
        .optional()?;
    Ok(user)
}

/// Deletes a user by ID. Returns `true` if a row was deleted.
pub async fn delete(db: &Db, id: Uuid) -> Result<bool, AppError> {
    let mut conn = db.get().await.map_err(pool_error)?;
    let deleted = diesel::delete(users::table.find(id)).execute(&mut conn).await?;
    Ok(deleted > 0)
}
