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
        tracing::warn!("Default TOKEN_SECRET used : do not use in prod");
    }

    let db = f4_repo::connect(&config.database_url).await.expect("connexion database impossible");
    f4_repo::migrate(&config.database_url).await.expect("migrations impossibles");

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let app = f1_router::router(AppState { db, config: Arc::new(config) });

    let listener = TcpListener::bind(addr).await.expect("bind impossible");
    tracing::info!("backend_rust écoute sur {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Error server");
    tracing::info!("Good finish");
}

/// Waits for `Ctrl-C` (`SIGINT`) or `SIGTERM` (sent by `docker stop`) for graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("listen Ctrl-C impossible");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("listen SIGTERM impossible")
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
