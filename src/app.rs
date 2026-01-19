use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use moka::future::Cache;
use sqids::Sqids;
use sqlx::{Pool, Postgres, Transaction, postgres::PgPoolOptions};
use thiserror::Error;
use tokio::net::TcpListener;

use crate::{api, config::Settings, domain::Url, metrics::Metrics, tasks};

#[derive(Debug, Clone)]
pub struct CachedLink {
    pub id: i64,
    pub url: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<Postgres>,
    sqids: Sqids,
    pub metrics: Arc<Metrics>,
    pub cache: Cache<String, Option<CachedLink>>,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("alias does not exist")]
    NotExists(String),
    #[error("database error {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppState {
    /// Shorten the provided URL and return the generated alias
    #[tracing::instrument(name = "app::shorten_url", skip(self))]
    pub async fn shorten_url(&self, url: &str) -> Result<String, AppError> {
        let mut tx: Transaction<Postgres> =
            self.pool.begin().await.map_err(AppError::DatabaseError)?;

        // Insert the url into database to get a unique id
        let rec = sqlx::query!(
            r#"
            INSERT INTO links_main (url)
            VALUES ($1)
            RETURNING id
            "#,
            url,
            expires_at,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let id = rec.id as u64;

        let alias = self
            .sqids
            .encode(&[id])
            .context("Sqids alphabet was exhausted")
            .map_err(AppError::Other)?;

        // Update the record with generated alias
        let updated = sqlx::query!(
            r#"
            UPDATE links_main
            SET alias = $1
            WHERE id = $2
            RETURNING alias
            "#,
            alias,
            rec.id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        tx.commit().await.map_err(AppError::DatabaseError)?;

        let alias = updated
            .alias
            .context("Updated record contained no alias")
            .map_err(AppError::Other)?;

        Ok(alias)
    }

    /// Query the database for the URL stored for the provided alias
    ///
    /// Returns Ok(None) if the alias does not exist
    pub async fn query_url(key: &str, pool: &Pool<Postgres>) -> Result<Option<CachedLink>> {
        let rec_opt = sqlx::query!(r#"SELECT id, url FROM links_main WHERE alias = $1"#, key)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        rec_opt
            .map(|rec| {
                let url = Url::parse(&rec.url)
                    .with_context(|| format!("Failed to validate url from {key}"))
                    .map_err(AppError::Other)?
                    .into_string();

                Ok(CachedLink { id: rec.id, url })
            })
            .transpose()
    }

    /// Save a user-defined alias for the provided URL
    ///
    /// Returns Ok(false) if the alias is already taken
    #[tracing::instrument(name = "app::save_named_url", skip(self))]
    pub async fn save_named_url(&self, alias: &str, url: &str) -> Result<bool, AppError> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO links_main (alias, url)
            VALUES ($1, $2)
            ON CONFLICT (alias) DO NOTHING
            RETURNING id
            "#,
            alias,
            url,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(rec.is_some())
    }
}

pub async fn connect_to_db(database_url: &str) -> Result<Pool<Postgres>> {
    // Connect to database
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("Failed to connect to database")?;

    // Run SQL migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("SQL migrations failed")?;

    Ok(pool)
}

pub async fn build_app_state(pool: Pool<Postgres>) -> Result<AppState> {
    const MIN_ALIAS_LENGTH: u8 = 6;
    // Shuffled alphabet for Sqids to generate ids from
    const ALPHABET: &str = "79Hr0JZijqWTnxhgoDEKMRpX4FNIfywG3e6LcldO5bCUYSBPa81s2QAumtzVvk";

    // Initialize Sqids generator
    let sqids = Sqids::builder()
        .min_length(MIN_ALIAS_LENGTH)
        .alphabet(ALPHABET.chars().collect())
        .build()?;

    let metrics = Arc::new(Metrics::new());

    {
        let pool = pool.clone();
        tokio::spawn(tasks::daily_partition::run(pool));
    }

    // TODO: notify?
    {
        let pool = pool.clone();
        let metrics = metrics.clone();
        tokio::spawn(tasks::flush_metrics::run(pool, metrics));
    }

    let cache: Cache<String, Option<CachedLink>> = Cache::new(1_000);

    Ok(AppState {
        pool,
        sqids,
        metrics,
        cache,
    })
}

pub async fn run(config: Settings) -> Result<()> {
    let pool = connect_to_db(config.database_url.as_str()).await?;
    let state = build_app_state(pool.clone()).await?;
    let router = api::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");

    axum::serve(listener, router).await?;

    Ok(())
}
