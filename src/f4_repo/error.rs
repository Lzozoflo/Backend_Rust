use crate::error::{conflict_message, AppError};

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db) if db.is_unique_violation() => {
                Self::Conflict(conflict_message(db.constraint()).to_string())
            }
            other => Self::Internal(other.to_string()),
        }
    }
}
