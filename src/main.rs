mod app_state;
mod auth;
mod infra;
mod rest;
mod web;

use std::{env, net::SocketAddr, path::Path};

use axum::serve;
use tokio::net::TcpListener;

use crate::{
    app_state::AppState,
    auth::AuthConfig,
    infra::db::{init_db, seed_demo_users},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/login_demo.db?mode=rwc".to_string());

    ensure_sqlite_parent_dir(&database_url)?;
    println!(
        "[Main] startup_begin | listen_addr={} | database_url={}",
        listen_addr, database_url
    );

    let auth_config = AuthConfig::from_env()?;
    let db_pool = init_db(&database_url).await?;
    seed_demo_users(&db_pool, &auth_config.demo_user_initial_password).await?;
    let app = web::build_router(AppState {
        db_pool,
        auth_config,
    });

    let socket_addr: SocketAddr = listen_addr.parse()?;
    let listener = TcpListener::bind(socket_addr).await?;
    println!("[Main] startup_ready | listen_addr={}", listen_addr);

    serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn ensure_sqlite_parent_dir(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = database_url.strip_prefix("sqlite:") {
        let db_file_path = path.split('?').next().unwrap_or(path);
        let db_path = Path::new(db_file_path);
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }
    Ok(())
}
