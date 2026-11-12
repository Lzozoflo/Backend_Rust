//! Helpers partagés par les tests d'intégration.
#![allow(dead_code)]
use std::sync::Arc;

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;
use backend_rust::{config::Config, f1_router, f4_repo, AppState};

pub const TEST_SECRET: &str = "test-secret-0123456789";

pub fn test_config() -> Config {
    Config {
        database_url: "postgres://u:p@127.0.0.1:1/none".into(),
        port: 0,
        app_version: "test".into(),
        token_secret: TEST_SECRET.into(),
        token_expiration: 60,
    }
}

pub fn app_with(db: f4_repo::Db, config: Config) -> Router {
    f1_router::router(AppState { db, config: Arc::new(config) })
}

/// Application dont la base n'est jamais jointe : suffisant pour tout ce qui est
/// rejeté avant la couche f4 (validation, authentification, routes inconnues).
pub fn app_without_db() -> Router {
    let config = test_config();
    app_with(f4_repo::connect_lazy(&config.database_url), config)
}

pub async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let request = match body {
        Some(json) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}
