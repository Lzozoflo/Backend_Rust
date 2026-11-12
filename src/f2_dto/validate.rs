use crate::error::AppError;

/// Input DTOs validate their own contents. The `ValidJson` extractor
/// (`f1_router/extract.rs`) calls `validate()` before the route handler executes,
/// ensuring invalid input never reaches the service layer.
pub trait Validate {
    fn validate(&self) -> Result<(), AppError>;
}
