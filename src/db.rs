use std::env;

use tokio::sync::OnceCell;
use dotenvy::dotenv;

static DB: OnceCell<sqlx::Pool<sqlx::Postgres>> = OnceCell::const_new();

pub async fn get() -> &'static sqlx::Pool<sqlx::Postgres> {
    DB.get_or_init(|| async {
        dotenv().ok();

        sqlx::PgPool::connect(&env::var("DATABASE_URL").unwrap())
            .await
            .unwrap()
    })
    .await
}
