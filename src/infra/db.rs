use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

use crate::auth::{hash_password, now_ts};

pub async fn init_db(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    println!("[DB] connect_begin | database_url={}", database_url);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    println!("[DB] migrate_begin | database_url={}", database_url);
    sqlx::migrate!("./migrations").run(&pool).await?;
    println!("[DB] migrate_done | database_url={}", database_url);

    Ok(pool)
}

pub async fn seed_demo_users(
    pool: &SqlitePool,
    initial_password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = now_ts();
    let password_hash = hash_password(initial_password).map_err(|error| {
        std::io::Error::other(format!("hash demo password failed: {error}"))
    })?;
    let demo_emails = ["test1@brain.im", "test2@brain.im"];

    println!(
        "[DB] seed_demo_users_begin | count={} | now={}",
        demo_emails.len(),
        now
    );

    for email in demo_emails {
        sqlx::query(
            r#"
            INSERT INTO users (email, password_hash, status, created_at, updated_at)
            VALUES (?, ?, 'active', ?, ?)
            ON CONFLICT(email) DO UPDATE SET
                password_hash = excluded.password_hash,
                status = 'active',
                updated_at = excluded.updated_at
            "#,
        )
        .bind(email)
        .bind(&password_hash)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    println!(
        "[DB] seed_demo_users_done | count={} | now={}",
        demo_emails.len(),
        now
    );
    Ok(())
}
