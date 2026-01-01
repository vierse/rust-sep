use anyhow::Result;
use async_trait::async_trait;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

const INIT_QUERY: &str = r#"
CREATE TABLE IF NOT EXISTS aliases (
    alias TEXT PRIMARY KEY,
    url   TEXT NOT NULL
);
"#;

#[async_trait]
pub trait Database {
    async fn insert(&self, alias: &str, url: &str) -> Result<()>;

    async fn get(&self, alias: &str) -> Result<String>;

    async fn exists(&self, alias: &str) -> Result<bool>;

    async fn remove(&self, alias: &str) -> Result<bool>;
}

#[derive(Clone)]
pub struct SqliteDB {
    pub pool: SqlitePool,
}

impl SqliteDB {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(database_url)
            .await?;

        sqlx::query(INIT_QUERY).execute(&pool).await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl Database for SqliteDB {
    async fn insert(&self, alias: &str, url: &str) -> Result<()> {
        sqlx::query("INSERT INTO aliases (alias, url) VALUES (?, ?)")
            .bind(alias)
            .bind(url)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get(&self, alias: &str) -> Result<String> {
        let url = sqlx::query_scalar::<_, String>("SELECT url FROM aliases WHERE alias = ?")
            .bind(alias)
            .fetch_one(&self.pool)
            .await?;

        Ok(url)
    }

    async fn exists(&self, alias: &str) -> Result<bool> {
        let exist = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT 1
            FROM aliases
            WHERE alias = ? ;
           "#,
        )
        .bind(alias)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exist.is_some())
    }

    async fn remove(&self, alias: &str) -> Result<bool> {
        let succesfully_removed = sqlx::query(
            r#"
            DELETE FROM aliases
            WHERE alias = ?;
            "#,
        )
        .bind(alias)
        .execute(&self.pool)
        .await?;

        Ok(succesfully_removed.rows_affected() > 0)
    }
}
