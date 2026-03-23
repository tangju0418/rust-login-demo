CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'disabled')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    issued_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    revoked_at INTEGER,
    user_agent TEXT,
    ip TEXT,
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_session_id ON refresh_tokens(session_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);

CREATE TABLE IF NOT EXISTS risk_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    dimension TEXT NOT NULL CHECK (dimension IN ('email', 'ip', 'email_ip')),
    dimension_key TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('success', 'failure')),
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_risk_events_dimension_key_created_at
    ON risk_events(dimension, dimension_key, created_at);

CREATE TABLE IF NOT EXISTS risk_states (
    dimension TEXT NOT NULL CHECK (dimension IN ('email', 'ip', 'email_ip')),
    dimension_key TEXT NOT NULL,
    fail_count INTEGER NOT NULL,
    window_start_at INTEGER NOT NULL,
    locked_until INTEGER,
    PRIMARY KEY (dimension, dimension_key)
);

CREATE INDEX IF NOT EXISTS idx_risk_states_locked_until ON risk_states(locked_until);
