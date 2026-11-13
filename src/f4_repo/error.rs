use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::error::{conflict_message, AppError};

impl From<DieselError> for AppError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::DatabaseError(DatabaseErrorKind::UniqueViolation, info) => {
                Self::Conflict(conflict_message(info.constraint_name()).to_string())
            }
            other => Self::Internal(other.to_string()),
        }
    }
}

/// Failure to get a connection from the pool (database down, pool exhausted…).
pub(super) fn pool_error(err: impl std::fmt::Display) -> AppError {
    AppError::Internal(format!("pool: {err}"))
}
