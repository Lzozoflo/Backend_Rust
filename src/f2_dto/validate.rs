use crate::error::AppError;

/// Un DTO d'entrée sait se valider lui-même. L'extracteur `ValidJson`
/// (f1_route/extract.rs) appelle `validate()` avant que le handler ne s'exécute :
/// une entrée invalide n'atteint donc jamais les services.
pub trait Validate {
    fn validate(&self) -> Result<(), AppError>;
}
