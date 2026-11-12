//! Tests d'intégration HTTP sans base de données.
mod common;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use common::{app_without_db, send, TEST_SECRET};
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;
use backend_rust::f3_services::user::token::create_token;

#[tokio::test]
async fn health_returns_ok_and_version() {
    let (status, body) = send(&app_without_db(), Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "status": "ok", "version": "test" }));
}

#[tokio::test]
async fn unknown_route_returns_json_404() {
    let (status, body) = send(&app_without_db(), Method::GET, "/nope", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "route inconnue");
}

#[tokio::test]
async fn cors_preflight_is_permissive() {
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/register")
        .header(header::ORIGIN, "http://localhost:3000")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .body(Body::empty())
        .unwrap();
    let response = app_without_db().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[tokio::test]
async fn register_with_invalid_payload_is_rejected_by_the_dto() {
    let payload = json!({ "username": "a", "email": "nope", "password": "x" });
    let (status, body) = send(&app_without_db(), Method::POST, "/register", None, Some(payload)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("username"));
}

#[tokio::test]
async fn register_with_missing_field_returns_json_400() {
    let (status, body) =
        send(&app_without_db(), Method::POST, "/register", None, Some(json!({ "username": "abc" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().starts_with("JSON invalide"));
}

#[tokio::test]
async fn login_with_empty_fields_is_400() {
    let payload = json!({ "email": "", "password": "" });
    let (status, _) = send(&app_without_db(), Method::POST, "/login", None, Some(payload)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn protected_routes_require_a_token() {
    let app = app_without_db();
    let (status, _) = send(&app, Method::PUT, "/user", None, Some(json!({ "username": "abc" }))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = send(&app, Method::DELETE, "/user", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn token_signed_with_another_secret_is_rejected() {
    let forged = create_token(Uuid::new_v4(), "pas-le-bon-secret", 60).unwrap();
    let (status, _) = send(&app_without_db(), Method::DELETE, "/user", Some(&forged), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn update_with_valid_token_but_empty_body_is_rejected_before_the_db() {
    let token = create_token(Uuid::new_v4(), TEST_SECRET, 60).unwrap();
    let (status, body) = send(&app_without_db(), Method::PUT, "/user", Some(&token), Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "aucun champ à modifier");
}
