// ! Erreur applicative unique, convertie en réponse JSON `{ "error": "..." }`.
// ! Les conversions depuis les erreurs de base de données vivent dans la couche f4.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Entrée invalide (rejetée par un DTO ou JSON mal formé).
    #[error("{0}")]
    Validation(String),
    #[error("identifiants invalides")]
    InvalidCredentials,
    #[error("non autorisé")]
    Unauthorized,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    NotFound(&'static str),
    /// Détail loggé côté serveur, jamais renvoyé au client.
    #[error("erreur interne")]
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
            tracing::error!(%detail, "erreur interne");
        }
        (self.status(), Json(json!({ "error": self.to_string() }))).into_response()
    }
}

/// Message de conflit selon la contrainte d'unicité violée.
pub fn conflict_message(constraint: Option<&str>) -> &'static str {
    match constraint {
        Some(c) if c.contains("email") => "email déjà utilisé",
        Some(c) if c.contains("username") => "username déjà utilisé",
        _ => "username ou email déjà utilisé",
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
        assert_eq!(AppError::Internal("mot de passe db".into()).to_string(), "erreur interne");
    }

    #[test]
    fn conflict_message_follows_constraint() {
        assert_eq!(conflict_message(Some("users_email_key")), "email déjà utilisé");
        assert_eq!(conflict_message(Some("users_username_key")), "username déjà utilisé");
        assert_eq!(conflict_message(None), "username ou email déjà utilisé");
    }
}
