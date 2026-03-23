use std::{collections::HashMap, net::SocketAddr};

use axum::{
    Json,
    body::Bytes,
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;
use serde_json::{Map, Value, json};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::{
        ACCESS_TOKEN_EXPIRES_IN, EMAIL_IP_LOCK_SECONDS, EMAIL_IP_LOCK_THRESHOLD, EMAIL_LOCK_SECONDS,
        EMAIL_LOCK_THRESHOLD, IP_LOCK_SECONDS, IP_LOCK_THRESHOLD, REFRESH_TOKEN_EXPIRES_IN,
        RISK_WINDOW_SECONDS, generate_refresh_token, hash_token, is_valid_email, normalize_email,
        now_ts, verify_password,
    },
};

const INVALID_PARAMS: &str = "invalid_params";
const INVALID_PARAMS_MESSAGE: &str = "invalid params";
const RATE_LIMITED: &str = "rate_limited";
const RATE_LIMITED_MESSAGE: &str = "too many attempts, try again later";
const AUTH_FAILED: &str = "auth_failed";
const AUTH_FAILED_MESSAGE: &str = "authentication failed";
const REFRESH_TOKEN_INVALID: &str = "refresh_token_invalid";
const REFRESH_TOKEN_INVALID_MESSAGE: &str = "refresh token invalid";

#[derive(Serialize)]
pub struct LoginResult {
    access_token: String,
    access_token_expires_in: i64,
    refresh_token: String,
    refresh_token_expires_in: i64,
}

#[derive(Serialize)]
pub struct RefreshResult {
    access_token: String,
    access_token_expires_in: i64,
    refresh_token: String,
    refresh_token_expires_in: i64,
}

#[derive(Serialize)]
pub struct CurrentUserResult {
    id: i64,
    email: String,
    status: String,
}

#[derive(Serialize)]
struct ErrorResult {
    name: &'static str,
    message: &'static str,
    data: Value,
}

enum SelectedValue {
    Present(String),
    Invalid,
}

pub async fn create_login(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> impl IntoResponse {
    let body_map = match parse_body_object(&body) {
        Ok(value) => value,
        Err(_) => {
            println!("[Auth] login_invalid_body | reason=body_not_json_object");
            return invalid_params_response();
        }
    };

    let raw_email = match select_string_field("email", &query, &body_map, &headers, None) {
        Ok(value) => value,
        Err(_) => return invalid_params_response(),
    };
    let raw_password = match select_string_field("password", &query, &body_map, &headers, None) {
        Ok(value) => value,
        Err(_) => return invalid_params_response(),
    };
    let ip = match select_string_field(
        "ip",
        &query,
        &body_map,
        &headers,
        Some(remote_addr.ip().to_string()),
    ) {
        Ok(value) => sanitize_optional_text(value),
        Err(_) => return invalid_params_response(),
    };
    let user_agent = match select_string_field("user_agent", &query, &body_map, &headers, None) {
        Ok(value) => sanitize_optional_text(value),
        Err(_) => return invalid_params_response(),
    };

    let now = now_ts();
    let normalized_email = raw_email.as_deref().and_then(normalize_email);
    let email_valid = normalized_email
        .as_deref()
        .map(is_valid_email)
        .unwrap_or(false);
    let password_valid = raw_password
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let risk_email = if email_valid {
        normalized_email.clone()
    } else {
        None
    };

    println!(
        "[Auth] login_enter | email={} | ip={} | has_user_agent={}",
        risk_email.as_deref().unwrap_or("-"),
        ip.as_deref().unwrap_or("-"),
        user_agent.is_some()
    );

    if raw_email.is_none() || !email_valid || raw_password.is_none() || !password_valid {
        if let Err(error) =
            record_failed_login_attempt(&state.db_pool, risk_email.as_deref(), ip.as_deref(), now).await
        {
            return internal_error_response("login_record_invalid_params_failure", &error.to_string());
        }
        println!(
            "[Auth] login_rejected | result={} | email={} | ip={}",
            INVALID_PARAMS,
            risk_email.as_deref().unwrap_or("-"),
            ip.as_deref().unwrap_or("-")
        );
        return invalid_params_response();
    }

    let email = normalized_email.expect("validated email must exist");
    let password = raw_password.expect("validated password must exist");

    match check_risk_lock(&state.db_pool, Some(email.as_str()), ip.as_deref(), now).await {
        Ok(Some(retry_after_seconds)) => {
            println!(
                "[Auth] login_rejected | result={} | email={} | ip={} | retry_after_seconds={}",
                RATE_LIMITED,
                email,
                ip.as_deref().unwrap_or("-"),
                retry_after_seconds
            );
            return rate_limited_response(retry_after_seconds);
        }
        Ok(None) => {}
        Err(error) => {
            return internal_error_response("login_check_risk_lock_failed", &error.to_string());
        }
    }

    let user_row = match sqlx::query(
        "SELECT id, email, password_hash, status FROM users WHERE email = ? LIMIT 1",
    )
    .bind(&email)
    .fetch_optional(&state.db_pool)
    .await
    {
        Ok(value) => value,
        Err(error) => return internal_error_response("login_fetch_user_failed", &error.to_string()),
    };

    let Some(user_row) = user_row else {
        if let Err(error) =
            record_failed_login_attempt(&state.db_pool, Some(email.as_str()), ip.as_deref(), now).await
        {
            return internal_error_response("login_record_missing_user_failure", &error.to_string());
        }
        println!(
            "[Auth] login_failed | result={} | email={} | ip={} | reason=user_not_found",
            AUTH_FAILED,
            email,
            ip.as_deref().unwrap_or("-")
        );
        return auth_failed_response();
    };

    let user_id: i64 = user_row.get("id");
    let user_status: String = user_row.get("status");
    let password_hash: String = user_row.get("password_hash");

    if user_status != "active" || !verify_password(&password, &password_hash) {
        if let Err(error) =
            record_failed_login_attempt(&state.db_pool, Some(email.as_str()), ip.as_deref(), now).await
        {
            return internal_error_response("login_record_auth_failure", &error.to_string());
        }
        println!(
            "[Auth] login_failed | result={} | email={} | ip={} | reason=credential_or_status_invalid",
            AUTH_FAILED,
            email,
            ip.as_deref().unwrap_or("-")
        );
        return auth_failed_response();
    }

    let session_id = Uuid::new_v4().to_string();
    let refresh_token = generate_refresh_token();
    let refresh_token_hash = hash_token(&refresh_token);
    let access_token = match state
        .auth_config
        .issue_access_token(user_id, &session_id, now)
    {
        Ok(value) => value,
        Err(error) => return internal_error_response("login_issue_access_token_failed", &error.to_string()),
    };

    let insert_result = sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, session_id, token_hash, issued_at, expires_at, revoked_at, user_agent, ip)
        VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(&session_id)
    .bind(&refresh_token_hash)
    .bind(now)
    .bind(now + REFRESH_TOKEN_EXPIRES_IN)
    .bind(user_agent.as_deref())
    .bind(ip.as_deref())
    .execute(&state.db_pool)
    .await;

    if let Err(error) = insert_result {
        return internal_error_response("login_store_refresh_token_failed", &error.to_string());
    }

    if let Err(error) = record_success_login_attempt(&state.db_pool, email.as_str(), ip.as_deref(), now).await
    {
        return internal_error_response("login_record_success_failed", &error.to_string());
    }

    println!(
        "[Auth] login_success | user_id={} | email={} | session_id={} | ip={}",
        user_id,
        email,
        session_id,
        ip.as_deref().unwrap_or("-")
    );

    (
        StatusCode::OK,
        Json(LoginResult {
            access_token,
            access_token_expires_in: ACCESS_TOKEN_EXPIRES_IN,
            refresh_token,
            refresh_token_expires_in: REFRESH_TOKEN_EXPIRES_IN,
        }),
    )
        .into_response()
}

pub async fn update_refresh_token(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let body_map = match parse_body_object(&body) {
        Ok(value) => value,
        Err(_) => {
            println!("[Auth] refresh_invalid_body | reason=body_not_json_object");
            return invalid_params_response();
        }
    };

    let refresh_token = match select_required_token_field("refresh_token", &query, &body_map, &headers) {
        Ok(value) => value,
        Err(_) => return invalid_params_response(),
    };
    let refresh_token_hash = hash_token(&refresh_token);
    let now = now_ts();

    println!(
        "[Auth] refresh_enter | refresh_token_hash={} | now={}",
        refresh_token_hash,
        now
    );

    let mut tx = match state.db_pool.begin().await {
        Ok(value) => value,
        Err(error) => return internal_error_response("refresh_begin_tx_failed", &error.to_string()),
    };

    let refresh_row = match sqlx::query(
        r#"
        SELECT rt.id, rt.user_id, rt.session_id, rt.expires_at, rt.revoked_at, u.email, u.status
        FROM refresh_tokens rt
        JOIN users u ON u.id = rt.user_id
        WHERE rt.token_hash = ?
        LIMIT 1
        "#,
    )
    .bind(&refresh_token_hash)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(value) => value,
        Err(error) => return internal_error_response("refresh_fetch_record_failed", &error.to_string()),
    };

    let Some(refresh_row) = refresh_row else {
        println!(
            "[Auth] refresh_failed | result={} | reason=token_not_found",
            REFRESH_TOKEN_INVALID
        );
        return refresh_token_invalid_response();
    };

    let record_id: String = refresh_row.get("id");
    let user_id: i64 = refresh_row.get("user_id");
    let session_id: String = refresh_row.get("session_id");
    let expires_at: i64 = refresh_row.get("expires_at");
    let revoked_at: Option<i64> = refresh_row.get("revoked_at");
    let email: String = refresh_row.get("email");
    let status: String = refresh_row.get("status");

    if revoked_at.is_some() || expires_at <= now {
        println!(
            "[Auth] refresh_failed | result={} | user_id={} | session_id={} | reason=token_inactive",
            REFRESH_TOKEN_INVALID,
            user_id,
            session_id
        );
        return refresh_token_invalid_response();
    }

    if status != "active" {
        println!(
            "[Auth] refresh_failed | result={} | user_id={} | session_id={} | reason=user_not_active",
            AUTH_FAILED,
            user_id,
            session_id
        );
        return auth_failed_response();
    }

    let revoke_result = match sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL AND expires_at > ?",
    )
    .bind(now)
    .bind(&record_id)
    .bind(now)
    .execute(&mut *tx)
    .await
    {
        Ok(value) => value,
        Err(error) => return internal_error_response("refresh_revoke_old_token_failed", &error.to_string()),
    };

    if revoke_result.rows_affected() == 0 {
        println!(
            "[Auth] refresh_failed | result={} | user_id={} | session_id={} | reason=concurrent_reuse",
            REFRESH_TOKEN_INVALID,
            user_id,
            session_id
        );
        return refresh_token_invalid_response();
    }

    let new_refresh_token = generate_refresh_token();
    let new_refresh_token_hash = hash_token(&new_refresh_token);
    let access_token = match state
        .auth_config
        .issue_access_token(user_id, &session_id, now)
    {
        Ok(value) => value,
        Err(error) => return internal_error_response("refresh_issue_access_token_failed", &error.to_string()),
    };

    let insert_result = sqlx::query(
        r#"
        INSERT INTO refresh_tokens (id, user_id, session_id, token_hash, issued_at, expires_at, revoked_at, user_agent, ip)
        VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, NULL)
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(&session_id)
    .bind(&new_refresh_token_hash)
    .bind(now)
    .bind(now + REFRESH_TOKEN_EXPIRES_IN)
    .execute(&mut *tx)
    .await;

    if let Err(error) = insert_result {
        return internal_error_response("refresh_insert_new_token_failed", &error.to_string());
    }

    if let Err(error) = tx.commit().await {
        return internal_error_response("refresh_commit_failed", &error.to_string());
    }

    println!(
        "[Auth] refresh_success | user_id={} | email={} | session_id={}",
        user_id,
        email,
        session_id
    );

    (
        StatusCode::OK,
        Json(RefreshResult {
            access_token,
            access_token_expires_in: ACCESS_TOKEN_EXPIRES_IN,
            refresh_token: new_refresh_token,
            refresh_token_expires_in: REFRESH_TOKEN_EXPIRES_IN,
        }),
    )
        .into_response()
}

pub async fn get_current_user(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let access_token = match select_required_token_field("access_token", &query, &Map::new(), &headers) {
        Ok(value) => value,
        Err(_) => return invalid_params_response(),
    };

    println!("[Auth] current_user_enter | has_access_token=true");

    let claims = match state.auth_config.decode_access_token(&access_token) {
        Ok(value) => value,
        Err(error) => {
            println!(
                "[Auth] current_user_failed | result={} | reason=decode_failed | error={}",
                AUTH_FAILED,
                error
            );
            return auth_failed_response();
        }
    };

    if claims.token_type != "access" {
        println!(
            "[Auth] current_user_failed | result={} | reason=invalid_token_type | token_type={}",
            AUTH_FAILED,
            claims.token_type
        );
        return auth_failed_response();
    }

    let user_id = match claims.sub.parse::<i64>() {
        Ok(value) => value,
        Err(_) => {
            println!(
                "[Auth] current_user_failed | result={} | reason=invalid_subject",
                AUTH_FAILED
            );
            return auth_failed_response();
        }
    };

    let user_row = match sqlx::query("SELECT id, email, status FROM users WHERE id = ? LIMIT 1")
        .bind(user_id)
        .fetch_optional(&state.db_pool)
        .await
    {
        Ok(value) => value,
        Err(error) => return internal_error_response("current_user_fetch_failed", &error.to_string()),
    };

    let Some(user_row) = user_row else {
        println!(
            "[Auth] current_user_failed | result={} | user_id={} | reason=user_not_found",
            AUTH_FAILED,
            user_id
        );
        return auth_failed_response();
    };

    let status: String = user_row.get("status");
    if status != "active" {
        println!(
            "[Auth] current_user_failed | result={} | user_id={} | reason=user_not_active",
            AUTH_FAILED,
            user_id
        );
        return auth_failed_response();
    }

    let email: String = user_row.get("email");
    println!(
        "[Auth] current_user_success | user_id={} | email={}",
        user_id,
        email
    );

    (
        StatusCode::OK,
        Json(CurrentUserResult {
            id: user_id,
            email,
            status,
        }),
    )
        .into_response()
}

fn parse_body_object(body: &Bytes) -> Result<Map<String, Value>, ()> {
    if body.is_empty() {
        return Ok(Map::new());
    }

    match serde_json::from_slice::<Value>(body) {
        Ok(Value::Object(map)) => Ok(map),
        _ => Err(()),
    }
}

fn select_required_token_field(
    field_name: &str,
    query: &HashMap<String, String>,
    body_map: &Map<String, Value>,
    headers: &HeaderMap,
) -> Result<String, ()> {
    let value = select_string_field(field_name, query, body_map, headers, None)?;
    match value {
        Some(token) if !token.trim().is_empty() => Ok(token),
        _ => Err(()),
    }
}

fn select_string_field(
    field_name: &str,
    query: &HashMap<String, String>,
    body_map: &Map<String, Value>,
    headers: &HeaderMap,
    default: Option<String>,
) -> Result<Option<String>, ()> {
    for candidate in [
        select_from_query(field_name, query),
        select_from_body(field_name, body_map),
        select_from_headers(field_name, headers),
        default.map(SelectedValue::Present),
    ] {
        match candidate {
            None => continue,
            Some(SelectedValue::Present(value)) => return Ok(Some(value)),
            Some(SelectedValue::Invalid) => return Err(()),
        }
    }

    Ok(None)
}

fn select_from_query(field_name: &str, query: &HashMap<String, String>) -> Option<SelectedValue> {
    query
        .get(field_name)
        .map(|value| SelectedValue::Present(value.clone()))
}

fn select_from_body(field_name: &str, body_map: &Map<String, Value>) -> Option<SelectedValue> {
    body_map.get(field_name).map(|value| match value {
        Value::String(content) => SelectedValue::Present(content.clone()),
        Value::Null => SelectedValue::Invalid,
        _ => SelectedValue::Invalid,
    })
}

fn select_from_headers(field_name: &str, headers: &HeaderMap) -> Option<SelectedValue> {
    let header_value = match field_name {
        "access_token" => headers
            .get("access_token")
            .or_else(|| headers.get("authorization")),
        "refresh_token" => headers.get("refresh_token"),
        "ip" => headers
            .get("ip")
            .or_else(|| headers.get("x-forwarded-for"))
            .or_else(|| headers.get("x-real-ip")),
        "user_agent" => headers.get("user_agent").or_else(|| headers.get("user-agent")),
        _ => headers.get(field_name),
    };

    header_value.map(|value| match value.to_str() {
        Ok(parsed) => {
            if field_name == "access_token" && parsed.to_ascii_lowercase().starts_with("bearer ") {
                SelectedValue::Present(parsed[7..].to_string())
            } else if field_name == "ip" {
                SelectedValue::Present(parsed.split(',').next().unwrap_or(parsed).trim().to_string())
            } else {
                SelectedValue::Present(parsed.to_string())
            }
        }
        Err(_) => SelectedValue::Invalid,
    })
}

fn sanitize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|content| {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn invalid_params_response() -> axum::response::Response {
    error_response(
        StatusCode::BAD_REQUEST,
        INVALID_PARAMS,
        INVALID_PARAMS_MESSAGE,
        json!({}),
    )
}

fn rate_limited_response(retry_after_seconds: i64) -> axum::response::Response {
    error_response(
        StatusCode::TOO_MANY_REQUESTS,
        RATE_LIMITED,
        RATE_LIMITED_MESSAGE,
        json!({ "retry_after_seconds": retry_after_seconds.max(0) }),
    )
}

fn auth_failed_response() -> axum::response::Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        AUTH_FAILED,
        AUTH_FAILED_MESSAGE,
        json!({}),
    )
}

fn refresh_token_invalid_response() -> axum::response::Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        REFRESH_TOKEN_INVALID,
        REFRESH_TOKEN_INVALID_MESSAGE,
        json!({}),
    )
}

fn internal_error_response(action: &str, error: &str) -> axum::response::Response {
    println!("[Auth] internal_error | action={} | error={}", action, error);
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        "internal server error",
        json!({}),
    )
}

fn error_response(
    status: StatusCode,
    name: &'static str,
    message: &'static str,
    data: Value,
) -> axum::response::Response {
    (status, Json(ErrorResult { name, message, data })).into_response()
}

fn email_ip_key(email: &str, ip: &str) -> String {
    format!("{}|{}", email, ip)
}

async fn check_risk_lock(
    db_pool: &SqlitePool,
    email: Option<&str>,
    ip: Option<&str>,
    now: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let mut retry_after_seconds = 0;

    if let Some(email) = email {
        retry_after_seconds = retry_after_seconds.max(read_lock_remaining(db_pool, "email", email, now).await?);
    }

    if let Some(ip) = ip {
        retry_after_seconds = retry_after_seconds.max(read_lock_remaining(db_pool, "ip", ip, now).await?);
    }

    if let (Some(email), Some(ip)) = (email, ip) {
        retry_after_seconds =
            retry_after_seconds.max(read_lock_remaining(db_pool, "email_ip", &email_ip_key(email, ip), now).await?);
    }

    if retry_after_seconds > 0 {
        Ok(Some(retry_after_seconds))
    } else {
        Ok(None)
    }
}

async fn read_lock_remaining(
    db_pool: &SqlitePool,
    dimension: &str,
    dimension_key: &str,
    now: i64,
) -> Result<i64, sqlx::Error> {
    let locked_until: Option<i64> = sqlx::query_scalar(
        "SELECT locked_until FROM risk_states WHERE dimension = ? AND dimension_key = ? LIMIT 1",
    )
    .bind(dimension)
    .bind(dimension_key)
    .fetch_optional(db_pool)
    .await?
    .flatten();

    Ok(locked_until.map(|value| (value - now).max(0)).unwrap_or(0))
}

async fn record_failed_login_attempt(
    db_pool: &SqlitePool,
    email: Option<&str>,
    ip: Option<&str>,
    now: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = db_pool.begin().await?;

    if let Some(ip) = ip {
        insert_risk_event(&mut tx, "ip", ip, "failure", now).await?;
        refresh_risk_state(&mut tx, "ip", ip, now, false).await?;
    }

    if let Some(email) = email {
        insert_risk_event(&mut tx, "email", email, "failure", now).await?;
        refresh_risk_state(&mut tx, "email", email, now, false).await?;

        if let Some(ip) = ip {
            let key = email_ip_key(email, ip);
            insert_risk_event(&mut tx, "email_ip", &key, "failure", now).await?;
            refresh_risk_state(&mut tx, "email_ip", &key, now, true).await?;
        }
    }

    tx.commit().await
}

async fn record_success_login_attempt(
    db_pool: &SqlitePool,
    email: &str,
    ip: Option<&str>,
    now: i64,
) -> Result<(), sqlx::Error> {
    let Some(ip) = ip else {
        return Ok(());
    };

    let key = email_ip_key(email, ip);
    let mut tx = db_pool.begin().await?;
    insert_risk_event(&mut tx, "email_ip", &key, "success", now).await?;
    sqlx::query(
        r#"
        INSERT INTO risk_states (dimension, dimension_key, fail_count, window_start_at, locked_until)
        VALUES (?, ?, 0, ?, NULL)
        ON CONFLICT(dimension, dimension_key) DO UPDATE SET
            fail_count = 0,
            window_start_at = excluded.window_start_at,
            locked_until = NULL
        "#,
    )
    .bind("email_ip")
    .bind(&key)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await
}

async fn insert_risk_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    dimension: &str,
    dimension_key: &str,
    outcome: &str,
    now: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO risk_events (dimension, dimension_key, outcome, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(dimension)
    .bind(dimension_key)
    .bind(outcome)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn refresh_risk_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    dimension: &str,
    dimension_key: &str,
    now: i64,
    consecutive_failures_only: bool,
) -> Result<(), sqlx::Error> {
    let window_start = now - RISK_WINDOW_SECONDS;
    let last_success: Option<i64> = if consecutive_failures_only {
        sqlx::query_scalar(
            r#"
            SELECT MAX(created_at)
            FROM risk_events
            WHERE dimension = ? AND dimension_key = ? AND outcome = 'success' AND created_at >= ?
            "#,
        )
        .bind(dimension)
        .bind(dimension_key)
        .bind(window_start)
        .fetch_one(&mut **tx)
        .await?
    } else {
        None
    };

    let fail_count: i64 = if let Some(last_success) = last_success {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM risk_events
            WHERE dimension = ? AND dimension_key = ? AND outcome = 'failure' AND created_at >= ? AND created_at > ?
            "#,
        )
        .bind(dimension)
        .bind(dimension_key)
        .bind(window_start)
        .bind(last_success)
        .fetch_one(&mut **tx)
        .await?
    } else {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(1)
            FROM risk_events
            WHERE dimension = ? AND dimension_key = ? AND outcome = 'failure' AND created_at >= ?
            "#,
        )
        .bind(dimension)
        .bind(dimension_key)
        .bind(window_start)
        .fetch_one(&mut **tx)
        .await?
    };

    let (threshold, lock_seconds) = match dimension {
        "email_ip" => (EMAIL_IP_LOCK_THRESHOLD, EMAIL_IP_LOCK_SECONDS),
        "email" => (EMAIL_LOCK_THRESHOLD, EMAIL_LOCK_SECONDS),
        "ip" => (IP_LOCK_THRESHOLD, IP_LOCK_SECONDS),
        _ => (i64::MAX, 0),
    };
    let locked_until = if fail_count >= threshold {
        Some(now + lock_seconds)
    } else {
        None
    };

    sqlx::query(
        r#"
        INSERT INTO risk_states (dimension, dimension_key, fail_count, window_start_at, locked_until)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(dimension, dimension_key) DO UPDATE SET
            fail_count = excluded.fail_count,
            window_start_at = excluded.window_start_at,
            locked_until = excluded.locked_until
        "#,
    )
    .bind(dimension)
    .bind(dimension_key)
    .bind(fail_count)
    .bind(window_start)
    .bind(locked_until)
    .execute(&mut **tx)
    .await?;

    println!(
        "[Risk] refresh_state | dimension={} | dimension_key={} | fail_count={} | locked_until={}",
        dimension,
        dimension_key,
        fail_count,
        locked_until.unwrap_or(0)
    );

    Ok(())
}
