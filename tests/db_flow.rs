//! Parcours complet contre une vraie base Postgres.
//! Ignoré par défaut : lancer avec `make test-db` (DATABASE_URL requis).
mod common;

use axum::http::{Method, StatusCode};
use common::{app_with, send, test_config};
use serde_json::{json, Value};
use backend_rust::{config::Config, f4_repo};
use uuid::Uuid;

#[tokio::test]
#[ignore = "nécessite Postgres : make test-db"]
async fn full_user_lifecycle() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL requis");
    f4_repo::migrate(&url).await.expect("migrations");
    let db = f4_repo::connect(&url).await.expect("connexion");
    let app = app_with(db, Config { database_url: url, ..test_config() });

    let id = Uuid::new_v4().simple().to_string();
    let username = format!("user_{}", &id[..10]);
    let email = format!("{username}@example.com");

    // ── register ──
    let payload = json!({ "username": username, "email": email, "password": "motdepasse1" });
    let (status, body) = send(&app, Method::POST, "/register", None, Some(payload.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["username"], username.as_str());
    assert!(body.get("password").is_none() && body.get("password_hash").is_none());

    // ── conflits (email, puis username sans tenir compte de la casse) ──
    let (status, body) = send(&app, Method::POST, "/register", None, Some(payload)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "email déjà utilisé");

    let same_name = json!({ "username": username.to_uppercase(), "email": format!("autre_{email}"), "password": "motdepasse1" });
    let (status, body) = send(&app, Method::POST, "/register", None, Some(same_name)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "username déjà utilisé");

    // ── login ──
    let login = |password: &str| json!({ "email": email.to_uppercase(), "password": password });
    let (status, body) = send(&app, Method::POST, "/login", None, Some(login("motdepasse1"))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let token = body["token"].as_str().unwrap().to_string();
    assert_eq!(body["token_type"], "Bearer");

    let (status, _) = send(&app, Method::POST, "/login", None, Some(login("mauvais-mdp1"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let unknown = json!({ "email": "inconnu@example.com", "password": "motdepasse1" });
    let (status, _) = send(&app, Method::POST, "/login", None, Some(unknown)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // ── update ──
    let new_name = format!("{username}_v2");
    let update = json!({ "username": new_name, "password": "nouveaumdp2" });
    let (status, body) = send(&app, Method::PUT, "/user", Some(&token), Some(update)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["username"], new_name.as_str());

    let (status, _) = send(&app, Method::POST, "/login", None, Some(login("motdepasse1"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "l'ancien mot de passe ne doit plus marcher");
    let (status, _) = send(&app, Method::POST, "/login", None, Some(login("nouveaumdp2"))).await;
    assert_eq!(status, StatusCode::OK);

    // ── delete ──
    let (status, _) = send(&app, Method::DELETE, "/user", Some(&token), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, body): (_, Value) = send(&app, Method::DELETE, "/user", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "utilisateur introuvable");
    let (status, _) = send(&app, Method::POST, "/login", None, Some(login("nouveaumdp2"))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
