use axum::Json;
use chrono::Utc;
use serde_json::json;

pub async fn get_echo() -> Json<serde_json::Value> {
    let now = Utc::now().to_rfc3339();
    println!("[Echo] get_enter | now={}", now);
    Json(json!({ "now": now }))
}
