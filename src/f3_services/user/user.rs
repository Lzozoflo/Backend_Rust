//! Business logic for the `user` entity. Calls the `f4_repo` layer and contains no SQL.
//! Input arrives pre-validated by the DTO layer.
use uuid::Uuid;

use super::{password, token};
use crate::{
    config::Config,
    error::AppError,
    f2_dto::{
        user_in::{normalize_email, normalize_username, LoginRequest, RegisterRequest, UpdateUserRequest},
        user_out::{AuthResponse, UserResponse},
    },
    f4_repo::{
        self,
        user::{NewUser, UserChanges},
        Db,
    },
};

const USER_NOT_FOUND: &str = "user not found";

pub async fn register(db: &Db, req: RegisterRequest) -> Result<UserResponse, AppError> {
    let password_hash = password::hash(req.password).await?;
    let email = normalize_email(&req.email);
    let new_user = NewUser {
        username: normalize_username(&req.username),
        email: &email,
        password_hash: &password_hash,
    };
    Ok(f4_repo::user::insert(db, new_user).await?.into())
}

pub async fn login(db: &Db, config: &Config, req: LoginRequest) -> Result<AuthResponse, AppError> {
    let Some(user) = f4_repo::user::find_by_email(db, &normalize_email(&req.email)).await? else {
        password::verify_dummy(req.password).await?;
        return Err(AppError::InvalidCredentials);
    };
    if !password::verify(req.password, user.password_hash.clone()).await? {
        return Err(AppError::InvalidCredentials);
    }

    Ok(AuthResponse {
        token: token::create_token(user.id, &config.token_secret, config.token_expiration)?,
        token_type: "Bearer",
        expires_in: config.token_expiration,
        user: user.into(),
    })
}

pub async fn update(db: &Db, id: Uuid, req: UpdateUserRequest) -> Result<UserResponse, AppError> {
    let password_hash = match req.password {
        Some(password) => Some(password::hash(password).await?),
        None => None,
    };
    let email = req.email.as_deref().map(normalize_email);
    let changes = UserChanges {
        username: req.username.as_deref().map(normalize_username),
        email: email.as_deref(),
        password_hash: password_hash.as_deref(),
    };

    let user = f4_repo::user::update(db, id, changes).await?.ok_or(AppError::NotFound(USER_NOT_FOUND))?;
    Ok(user.into())
}

pub async fn delete(db: &Db, id: Uuid) -> Result<(), AppError> {
    if f4_repo::user::delete(db, id).await? {
        Ok(())
    } else {
        Err(AppError::NotFound(USER_NOT_FOUND))
    }
}
