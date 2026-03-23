use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

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
