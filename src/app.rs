use anyhow::{Context, Result, bail};
use rand::{Rng, distributions::Alphanumeric};
use sqlx::{PgPool, Pool, Postgres};
use tokio::net::TcpListener;

use crate::{api, config::Settings};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
}

fn generate_alias() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect()
}

impl AppState {
    #[tracing::instrument(name = "app::shorten_url", skip(self))]
    pub async fn shorten_url(&self, url: &str) -> Result<String> {
        let alias = generate_alias();

        let rec = sqlx::query!(
            r#"
            INSERT INTO links (alias, url)
            VALUES ($1, $2)
            ON CONFLICT (alias) DO NOTHING
            RETURNING alias
            "#,
            alias,
            url
        )
        .fetch_optional(&self.pool)
        .await
        .context("DB insert query failed")?;

        match rec {
            Some(r) => Ok(r.alias),
            None => bail!("Alias already exists"),
        }
    }

    #[tracing::instrument(name = "app::get_url", skip(self))]
    pub async fn get_url(&self, alias: &str) -> Result<String> {
        let rec = sqlx::query!(
            r#"
            SELECT url
            FROM links
            WHERE alias = $1
            "#,
            alias
        )
        .fetch_optional(&self.pool)
        .await
        .context("DB select query failed")?;

        match rec {
            Some(r) => Ok(r.url),
            None => bail!("This alias does not exist"),
        }
    }
}

pub async fn run(config: Settings) -> Result<()> {
    let pool = PgPool::connect(config.database_url.as_str()).await?;
    let router = api::build_router(AppState { pool });

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
