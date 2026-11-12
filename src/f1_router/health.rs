//! # Routes health
//! 
//! ```
//! /heatlh   get(heatlh)
//! ```

use axum::{extract::State, Json, routing::get, Router};

use crate::{f2_dto::health_out::HealthResponse, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
}

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok", version: state.config.app_version.clone() })
}
