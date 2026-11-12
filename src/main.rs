use std::{net::SocketAddr, sync::Arc};

use tokio::net::TcpListener;
use backend_rust::{config::Config, f1_router, f4_repo, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let config = Config::from_env();
    if config.uses_dev_secret() {
        tracing::warn!("Default TOKEN_SECRET used: do not use in prod");
    }

    let db = f4_repo::connect(&config.database_url).await.expect("database connection failed");
    f4_repo::migrate(&config.database_url).await.expect("database migrations failed");

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let app = f1_router::router(AppState { db, config: Arc::new(config) });

    let listener = TcpListener::bind(addr).await.expect("failed to bind TCP listener");
    tracing::info!("backend_rust listening on {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
    tracing::info!("graceful shutdown complete");
}

/// Waits for `Ctrl-C` (`SIGINT`) or `SIGTERM` (sent by `docker stop`) for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to listen for Ctrl-C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}