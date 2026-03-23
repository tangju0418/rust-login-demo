use axum::{
    routing::{get, patch, post},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{app_state::AppState, rest};

pub fn build_router(app_state: AppState) -> Router {
    Router::new()
        .route("/health/get", get(rest::health::get_health))
        .route("/echo/get", get(rest::echo::get_echo))
        .route("/login/create", post(rest::auth::create_login))
        .route("/refresh_token/update", patch(rest::auth::update_refresh_token))
        .route("/current_user/get", get(rest::auth::get_current_user))
        .route_service("/", ServeFile::new("static/index.html"))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(app_state)
}
