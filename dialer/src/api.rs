pub mod routes;
pub mod websocket;

use std::sync::Arc;

use axum::Router;
use tokio::sync::{broadcast, mpsc};
use tower_http::cors::CorsLayer;

use crate::controller::commands::CallStateUpdate;
use crate::controller::ControllerCommand;
use crate::store::CallStore;

/// Shared state available to all API handlers.
#[derive(Clone)]
pub struct AppState {
    pub command_tx: mpsc::Sender<ControllerCommand>,
    pub state_tx: broadcast::Sender<CallStateUpdate>,
    pub store: Arc<dyn CallStore>,
}

/// Build the axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::call_routes())
        .merge(websocket::ws_routes())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
