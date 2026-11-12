//! Backend Rust — HTTP.
//!
//! Flux of one request : `f1_router` → `f2_dto` (validation) → `f3_services` → `f4_repo`.

pub mod config;
pub mod error;

pub mod f1_router;
pub mod f2_dto;
pub mod f3_services;
pub mod f4_repo;


use std::sync::Arc;

use crate::{config::Config, f4_repo::Db};

/// État partagé entre toutes les routes.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
}
