use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use moka::future::Cache;
use sqids::Sqids;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::net::TcpListener;

use crate::{api, config::Settings, metrics::Metrics, tasks};

#[derive(Debug, Clone)]
pub struct CachedLink {
    pub id: i64,
    pub url: String,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sqids: Arc<Sqids>,
    pub metrics: Arc<Metrics>,
    pub cache: Cache<String, Option<CachedLink>>,
}

pub async fn connect_to_db(database_url: &str) -> Result<PgPool> {
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

pub async fn build_app_state(pool: PgPool) -> Result<AppState> {
    const MIN_ALIAS_LENGTH: u8 = 6;
    // Shuffled alphabet for Sqids to generate ids from
    const ALPHABET: &str = "79Hr0JZijqWTnxhgoDEKMRpX4FNIfywG3e6LcldO5bCUYSBPa81s2QAumtzVvk";

    // Initialize Sqids generator
    let sqids = Arc::new(
        Sqids::builder()
            .min_length(MIN_ALIAS_LENGTH)
            .alphabet(ALPHABET.chars().collect())
            .build()?,
    );

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
