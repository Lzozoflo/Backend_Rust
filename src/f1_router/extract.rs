//! Extracteurs partagés par les routes.
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

/// Corps JSON désérialisé **et validé** : si le handler s'exécute, le DTO est valide.
/// Les erreurs de format sont renvoyées en JSON (400), comme les erreurs de validation.
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
            .map_err(|rejection| AppError::Validation(format!("JSON invalide : {}", rejection.body_text())))?;
        value.validate()?;
        Ok(Self(value))
    }
}

/// Utilisateur authentifié via `Authorization: Bearer <jwt>`.
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
        token::verify_token(jwt, &state.config.token_secret).map(Self)
    }
}
