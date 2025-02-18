use sqlx::{MySql, Pool};

pub async fn create_pool() -> Result<Pool<MySql>, crate::error::ApplicationError> {
    let database_url = std::env::var("DATABASE_URL")?;

    let pool = Pool::connect(&database_url).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
