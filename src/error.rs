//! Unified application error, converted into a `{ "error": "..." }` JSON response.
//! Conversions from database errors live in the `f4_repo` layer.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Invalid input (rejected by DTO validation or malformed JSON).
    #[error("{0}")]
    Validation(String),
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("unauthorized")]
    Unauthorized,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    NotFound(&'static str),
    /// Server-side logged detail, never exposed to the client.
    #[error("internal server error")]
    Internal(String),
}


impl AppError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::InvalidCredentials | Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Self::Internal(detail) = &self {
            tracing::error!(%detail, "internal server error");
        }
        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}

/// Returns the conflict message matching the violated unique constraint.
pub fn conflict_message(constraint: Option<&str>) -> &'static str {
    match constraint {
        Some(c) if c.contains("email") => "email already in use",
        Some(c) if c.contains("username") => "username already in use",
        _ => "username or email already in use",
    }
}

// ═══════════════════════════════ UNIT TEST ═══════════════════════════════
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_codes() {
        assert_eq!(AppError::Validation("x".into()).status(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::InvalidCredentials.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::Unauthorized.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AppError::Conflict("x".into()).status(), StatusCode::CONFLICT);
        assert_eq!(AppError::NotFound("x").status(), StatusCode::NOT_FOUND);
        assert_eq!(AppError::Internal("x".into()).status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn internal_detail_is_never_exposed() {
        assert_eq!(AppError::Internal("db password".into()).to_string(), "internal server error");
    }

    #[test]
    fn conflict_message_follows_constraint() {
        assert_eq!(conflict_message(Some("users_email_key")), "email already in use");
        assert_eq!(conflict_message(Some("users_username_key")), "username already in use");
        assert_eq!(conflict_message(None), "username or email already in use");
    }
}
