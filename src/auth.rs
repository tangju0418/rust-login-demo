use std::{env, io};

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::SaltString,
};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub const ACCESS_TOKEN_EXPIRES_IN: i64 = 7_200;
pub const REFRESH_TOKEN_EXPIRES_IN: i64 = 2_592_000;
pub const RISK_WINDOW_SECONDS: i64 = 600;
pub const EMAIL_IP_LOCK_THRESHOLD: i64 = 5;
pub const EMAIL_IP_LOCK_SECONDS: i64 = 900;
pub const EMAIL_LOCK_THRESHOLD: i64 = 10;
pub const EMAIL_LOCK_SECONDS: i64 = 1_800;
pub const IP_LOCK_THRESHOLD: i64 = 30;
pub const IP_LOCK_SECONDS: i64 = 600;

#[derive(Clone)]
pub struct AuthConfig {
    jwt_secret: String,
    pub demo_user_initial_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub session_id: String,
    pub jti: String,
    pub token_type: String,
    pub iat: usize,
    pub exp: usize,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, io::Error> {
        let jwt_secret = env::var("JWT_SECRET").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "JWT_SECRET is required for authentication",
            )
        })?;
        let demo_user_initial_password = env::var("DEMO_USER_INITIAL_PASSWORD").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "DEMO_USER_INITIAL_PASSWORD is required for demo users",
            )
        })?;

        Ok(Self {
            jwt_secret,
            demo_user_initial_password,
        })
    }

    pub fn issue_access_token(
        &self,
        user_id: i64,
        session_id: &str,
        now: i64,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            session_id: session_id.to_string(),
            jti: Uuid::new_v4().to_string(),
            token_type: "access".to_string(),
            iat: now as usize,
            exp: (now + ACCESS_TOKEN_EXPIRES_IN) as usize,
        };

        jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
    }

    pub fn decode_access_token(
        &self,
        token: &str,
    ) -> Result<AccessTokenClaims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::default();
        validation.validate_exp = true;
        validation.required_spec_claims.extend(["exp".into(), "iat".into()]);
        let token_data = jsonwebtoken::decode::<AccessTokenClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )?;
        Ok(token_data.claims)
    }
}

pub fn now_ts() -> i64 {
    Utc::now().timestamp()
}

pub fn normalize_email(input: &str) -> Option<String> {
    let normalized = input.trim().to_lowercase();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

pub fn is_valid_email(email: &str) -> bool {
    if email.contains(' ') {
        return false;
    }

    let mut parts = email.split('@');
    let local = match parts.next() {
        Some(value) if !value.is_empty() => value,
        _ => return false,
    };
    let domain = match parts.next() {
        Some(value) if !value.is_empty() => value,
        _ => return false,
    };

    if parts.next().is_some() {
        return false;
    }

    if domain.starts_with('.') || domain.ends_with('.') || !domain.contains('.') {
        return false;
    }

    !local.is_empty()
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(value) => value,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn generate_refresh_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn hash_token(token: &str) -> String {
    format!("{:x}", Sha256::digest(token.as_bytes()))
}
