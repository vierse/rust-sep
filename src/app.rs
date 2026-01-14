use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use sqids::Sqids;
use sqlx::{Pool, Postgres, Transaction, postgres::PgPoolOptions};
use tokio::net::TcpListener;

use crate::{api, config::Settings, maintenance::{UsageMetrics, DefaultUsageMetrics, Cache, NoOpCache, MaintenanceScheduler, tasks::CleanupUnusedLinksTask}};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
    sqids: Sqids,
    usage_metrics: Arc<dyn UsageMetrics>,
}

impl AppState {
    #[tracing::instrument(name = "app::shorten_url", skip(self))]
    pub async fn shorten_url(&self, url: &str) -> Result<String> {
        let mut tx: Transaction<Postgres> = self.pool.begin().await?;

        // Insert the url into database to get a unique id
        let rec = sqlx::query!(
            r#"
            INSERT INTO links (url)
            VALUES ($1)
            RETURNING id
            "#,
            url,
        )
        .fetch_one(&mut *tx)
        .await
        .context("DB insert url query failed")?;

        let id = rec.id as u64;

        let alias = self.sqids.encode(&[id])?;

        // Update the record with generated alias
        let updated = sqlx::query!(
            r#"
            UPDATE links
            SET alias = $1
            WHERE id = $2
            RETURNING alias
            "#,
            alias,
            rec.id
        )
        .fetch_one(&mut *tx)
        .await
        .context("DB update url query failed")?;

        tx.commit()
            .await
            .context("DB failed to commit transaction")?;

        let alias = updated.alias.context("Alias was not set after update")?;

        Ok(alias)
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
            Some(r) => {
                // Record access for usage metrics
                if let Err(e) = self.usage_metrics.record_access(alias).await {
                    tracing::warn!(error = %e, alias = alias, "Failed to record access metrics");
                }
                Ok(r.url)
            }
            None => bail!("This alias does not exist"),
        }
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

    // Initialize usage metrics
    let usage_metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));

    Ok(AppState { pool, sqids, usage_metrics })
}

pub async fn run(config: Settings) -> Result<()> {
    let pool = connect_to_db(config.database_url.as_str()).await?;
    let state = build_app_state(pool.clone()).await?;
    
    // Set up maintenance scheduler
    let usage_metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));
    let cache: Arc<dyn Cache> = Arc::new(NoOpCache);
    
    let mut scheduler = MaintenanceScheduler::new(
        pool.clone(),
        usage_metrics.clone(),
        cache,
    );
    
    // Add maintenance tasks
    scheduler.add_task(Arc::new(CleanupUnusedLinksTask::default()));
    
    // Start scheduler in background
    let scheduler_handle = {
        let scheduler = scheduler;
        tokio::spawn(async move {
            if let Err(e) = scheduler.start().await {
                tracing::error!(error = %e, "Maintenance scheduler error");
            }
        })
    };
    
    let router = api::build_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");
    
    // Run server
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, router).await
    });
    
    // Wait for either server or scheduler to finish (they shouldn't)
    tokio::select! {
        result = server_handle => {
            result??;
        }
        _ = scheduler_handle => {
            tracing::warn!("Maintenance scheduler stopped unexpectedly");
        }
    }

    Ok(())
}
