pub mod routes;
pub mod state;
pub mod ws;

use axum::{routing::{get, post}, Router};
use crate::adapters::http::state::HttpState;
use crate::adapters::http::ws::ws_handler;

pub fn router(state: HttpState) -> Router {
    Router::new()
        .route("/api/config", get(routes::get_config))
        .route("/api/config", post(routes::apply_config))
        .route("/api/cameras", get(routes::list_cameras))
        .route("/api/cameras/:index/modes", get(routes::list_modes_by_index))
        .route("/api/cameras/:index/controls", get(routes::list_controls_by_index))
        .route("/api/cameras/:index/controls", post(routes::set_controls_by_index))
        .route("/api/files", get(routes::list_files)) // Nueva ruta
        .route("/ws/stream", get(ws_handler))
        .with_state(state)
}