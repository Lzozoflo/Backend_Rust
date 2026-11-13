//! HTTP Routing Layer (`f1_router`).

pub mod extract;
pub mod health;
pub mod user;

use axum::Router;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{error::AppError, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(user::router())
        .fallback(|| async { AppError::NotFound("unknown route") })
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
