use std::net::SocketAddr;

use axum::{
    Json,
    extract::ConnectInfo,
    http::HeaderMap,
};
use serde_json::json;

pub async fn get_request_context(
    headers: HeaderMap,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Json<serde_json::Value> {
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| remote_addr.ip().to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    println!(
        "[WebContext] get_request_context | ip={} | has_user_agent={}",
        ip,
        user_agent.is_some()
    );

    Json(json!({
        "ip": ip,
        "user_agent": user_agent,
    }))
}
