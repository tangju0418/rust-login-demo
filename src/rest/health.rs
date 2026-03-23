use axum::Json;
use serde_json::json;

pub async fn get_health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
