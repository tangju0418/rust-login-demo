use sqlx::SqlitePool;

use crate::auth::AuthConfig;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub auth_config: AuthConfig,
}
