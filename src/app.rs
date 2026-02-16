use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result};
use argon2::Argon2;
use moka::future::Cache;
use sqids::Sqids;
use sqlx::{PgPool, postgres::PgPoolOptions};
use time::Date;
use tokio::{net::TcpListener, time::timeout};
use tokio_util::sync::CancellationToken;
pub mod usage_metrics;

use crate::{
    api::{self, Sessions},
    config::Settings,
    domain::MIN_ALIAS_LENGTH,
    scheduler::Scheduler,
    tasks::{
        diag, link_cleanup,
        link_metrics::{self, LinkMetrics},
    },
};

#[derive(Debug, Clone)]
pub struct CachedLink {
    pub id: i64,
    pub url: String,
    pub last_seen: Date,
    pub password_hash: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sqids: Arc<Sqids>,
    pub usage_metrics: Arc<usage_metrics::Metrics>,
    pub metrics: Arc<LinkMetrics>,
    pub cache: Cache<String, Option<CachedLink>>,
    pub sessions: Sessions,
    pub hasher: Arc<Argon2<'static>>,
    pub diag: Arc<Diag>,
}

#[derive(Default)]
pub struct Diag {
    cache_hit: AtomicU64,
    cache_miss: AtomicU64,
}

impl Diag {
    #[inline]
    pub fn cache_hit(&self) {
        self.cache_hit.fetch_add(1, Ordering::Relaxed);
    }

    #[inline]
    pub fn cache_miss(&self) {
        self.cache_miss.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64) {
        (
            self.cache_hit.load(Ordering::Relaxed),
            self.cache_miss.load(Ordering::Relaxed),
        )
    }
}
pub async fn connect_to_db(database_url: &str) -> Result<PgPool> {
    // Connect to database
    let pool = PgPoolOptions::new()
        .min_connections(8)
        .max_connections(32)
        .max_lifetime(Duration::from_hours(1))
        .acquire_timeout(Duration::from_secs(15))
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

pub fn build_test_app_state(pool: PgPool) -> Result<AppState> {
    let metrics = Arc::new(LinkMetrics::new());
    build_app_state(pool, metrics)
}

pub fn build_app_state(pool: PgPool, metrics: Arc<LinkMetrics>) -> Result<AppState> {
    // Shuffled alphabet for Sqids to generate ids from
    const ALPHABET: &str = "79Hr0JZijqWTnxhgoDEKMRpX4FNIfywG3e6LcldO5bCUYSBPa81s2QAumtzVvk";

    // Initialize Sqids generator
    let sqids = Arc::new(
        Sqids::builder()
            .min_length(MIN_ALIAS_LENGTH as u8)
            .alphabet(ALPHABET.chars().collect())
            .build()?,
    );

    let cache: Cache<String, Option<CachedLink>> = Cache::builder()
        .time_to_idle(Duration::from_secs(60 * 60 * 24))
        .max_capacity(3_000)
        .build();

    Ok(AppState {
        pool,
        sqids,
        metrics,
        cache,
        sessions: Sessions::default(),
        hasher: Arc::new(Argon2::default()),
        usage_metrics: Default::default(),
        diag: Arc::new(Diag::default()),
    })
}

pub async fn run(config: Settings) -> Result<()> {
    let pool = connect_to_db(config.database_url.as_str()).await?;

    let metrics = Arc::new(LinkMetrics::new());

    let state = build_app_state(pool.clone(), metrics.clone())?;
    let diag = state.diag.clone();
    let router = api::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");

    let mut scheduler = Scheduler::new();

    scheduler.spawn_task(
        Scheduler::SECONDS_IN_DAY,
        "daily_partition",
        pool.clone(),
        |p| async move { link_metrics::create_partitions_task(p).await },
    );

    scheduler.spawn_task(
        15,
        "daily_metrics",
        (pool.clone(), metrics.clone()),
        |(p, m)| async move { link_metrics::process_batch_task(p, m).await },
    );

    scheduler.spawn_task(
        Scheduler::SECONDS_IN_DAY,
        "link_cleanup",
        pool.clone(),
        |p| async move { link_cleanup::link_cleanup_task(p).await },
    );

    scheduler.spawn_task(5, "diag", diag, |d| async move {
        diag::print_diagnostics_task(d).await
    });

    let cancel_main = CancellationToken::new();
    let server_handle = {
        let cancel = cancel_main.clone();
        let server = axum::serve(listener, router);
        tokio::spawn(async move {
            server
                .with_graceful_shutdown(cancel.cancelled_owned())
                .await
        })
    };

    wait_for_shutdown().await;
    cancel_main.cancel();

    let server_result = timeout(Duration::from_secs(60), server_handle).await;
    match server_result {
        Ok(result) => {
            tracing::info!("API shutdown successful");
            result??
        }
        Err(_) => tracing::error!("Timed out on shutdown"),
    }

    tracing::info!("Shutting down background tasks...");
    scheduler.shutdown(60).await;

    Ok(())
}

async fn wait_for_shutdown() {
    use tokio::signal::{
        self,
        unix::{SignalKind, signal},
    };

    let mut sig_term = signal(SignalKind::terminate()).expect("SIGTERM error");

    tokio::select! {
        _ = signal::ctrl_c() => {
            tracing::info!("Received Ctrl+C (SIGINT). Shutting down...");
        }
        _ = sig_term.recv() => {
            tracing::info!("Received SIGTERM. Shutting down...");
        }
    }
}
