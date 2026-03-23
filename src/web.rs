use axum::{routing::get, Router};
use tower_http::services::{ServeDir, ServeFile};

use crate::rest;

pub fn build_router() -> Router {
    Router::new()
        .route("/health/get", get(rest::health::get_health))
        .route("/echo/get", get(rest::echo::get_echo))
        .route_service("/", ServeFile::new("static/index.html"))
        .nest_service("/static", ServeDir::new("static"))
}
