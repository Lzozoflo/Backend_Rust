//! # User Routes
//!
//! ```text
//! POST   /register -> register()
//! POST   /login    -> login()
//! PUT    /user     -> update_user()
//! DELETE /user     -> delete_user()
//! ```

use axum::{extract::State, http::StatusCode, Json, routing::{post, put}, Router};

use super::extract::{AuthUser, ValidJson};
use crate::{
    error::AppError,
    f2_dto::{
        user_in::{LoginRequest, RegisterRequest, UpdateUserRequest},
        user_out::{AuthResponse, UserResponse},
    },
    f3_services::user as user_service,
    AppState,
};
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/user", put(update_user).delete(delete_user))
        .route("/user/logout-all", post(logout_all))
}




pub async fn register(
    State(state): State<AppState>,
    ValidJson(req): ValidJson<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    
    let user = user_service::register(&state.db, req).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn login(
    State(state): State<AppState>,
    ValidJson(req): ValidJson<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {

    Ok(Json(user_service::login(&state.db, &state.config, req).await?))
}

pub async fn update_user(
    State(state): State<AppState>,
    AuthUser(id): AuthUser,
    ValidJson(req): ValidJson<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {

    Ok(Json(user_service::update(&state.db, id, req).await?))
}

pub async fn delete_user(
    State(state): State<AppState>,
    AuthUser(id): AuthUser
) -> Result<StatusCode, AppError> {

    user_service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}


pub async fn logout_all(
    State(state): State<AppState>,
    AuthUser(id): AuthUser,
) -> Result<StatusCode, AppError> {
    user_service::logout_all(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}