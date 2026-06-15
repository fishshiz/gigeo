use sqlx::{Error, PgPool};

#[derive(Clone)]
pub struct AppDatabase {
    pub pool: PgPool,
}

impl AppDatabase {
    pub async fn new(database_url: &str) -> Result<Self, Error> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }
}
