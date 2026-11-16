//! Shared Axum extractors for route handlers.
//! 
use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts},
    Json,
};
use serde::de::DeserializeOwned;
use uuid::Uuid;

use crate::{
    error::AppError,
    f2_dto::validate::Validate,
    f3_services::user::token,
    AppState,
};


/// Deserialized **and validated** JSON body: if the handler runs, the DTO is guaranteed to be valid.
/// Formatting errors are returned as JSON (`400 Bad Request`), just like validation errors.
pub struct ValidJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;
    
    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|rejection| AppError::Validation(format!("invalid JSON: {}", rejection.body_text())))?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Authenticated user extracted from `Authorization: Bearer <jwt>`.
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jwt = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let (user_id, token_ver) = token::verify_token(jwt, &state.config.token_secret)?;

        // Vérification de la version en base de données
        let current_ver = crate::f4_repo::user::get_token_version(&state.db, user_id)
            .await?
            .ok_or(AppError::Unauthorized)?;

        if token_ver != current_ver {
            return Err(AppError::Unauthorized);
        }

        Ok(Self(user_id))
    }
}
