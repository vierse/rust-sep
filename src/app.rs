use anyhow::{Context, Result, bail};
use rand::{Rng, distributions::Alphanumeric};
use sqlx::{PgPool, Pool, Postgres};
use tokio::net::TcpListener;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::{api, config::Settings};

const MIN_ALIAS_LENGTH: usize = 6;

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
    alias_length: Arc<AtomicUsize>,
}

fn generate_alias(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

impl AppState {
    #[tracing::instrument(name = "app::shorten_url", skip(self))]
    pub async fn shorten_url(&self, url: &str) -> Result<String> {
        const MAX_RETRIES: usize = 5;

        let mut len = self.alias_length.load(Ordering::Relaxed);
        for _ in 0..MAX_RETRIES {
            let alias = generate_alias(len);

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

            if let Some(r) = rec {
                return Ok(r.alias);
            }

            len += 1;
            self.alias_length.fetch_add(1, Ordering::Relaxed);
        }

        bail!("Failed to generate a unique alias after {MAX_RETRIES} attempts");
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
    let state = AppState {
        pool,
        alias_length: Arc::new(AtomicUsize::new(MIN_ALIAS_LENGTH)),
    };
    let router = api::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
