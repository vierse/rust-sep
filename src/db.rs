use anyhow::{Context, Result};
use sqlx::{Pool, Postgres};

pub struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn remove(&self, alias: &str) -> Result<bool> {
        let rec = sqlx::query(
            r#"
            DELETE FROM links
            WHERE alias = $1
            "#,
        )
        .bind(alias)
        .execute(&self.pool)
        .await
        .context("connection failed while removing alias")?;

        Ok(rec.rows_affected() > 0)
    }

    pub async fn insert(&self, alias: &str, url: &str) -> Result<()> {
        sqlx::query("INSERT INTO links (alias, url) VALUES ($1, $2)")
            .bind(alias)
            .bind(url)
            .execute(&self.pool)
            .await
            .context("connection failed while inserting alias")?;

        Ok(())
    }
}
