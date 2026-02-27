use std::{env, time::Duration};

use anyhow::{Context, Result};
use metrics_exporter_prometheus::PrometheusBuilder;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, time::timeout};
use tokio_util::sync::CancellationToken;

use crate::{
    api::{self, AppKeys, AppState},
    scheduler::Scheduler,
    tasks::{link_cleanup, link_metrics},
};

pub async fn connect_to_db(database_url: &str) -> Result<PgPool> {
    // Connect to database
    let pool = PgPoolOptions::new()
        .min_connections(8)
        .max_connections(32)
        .max_lifetime(Duration::from_secs(60 * 60))
        .acquire_timeout(Duration::from_secs(15))
        .connect(database_url)
        .await
        .context("Failed to connect to database")?;

    // Run SQL migrations in case sqlx is using offline cache
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("SQL migrations failed")?;

    Ok(pool)
}

pub async fn run() -> Result<()> {
    let app_port: u16 = env::var("APP_PORT")
        .expect("APP_PORT not set in env")
        .parse()
        .expect("Could not parse APP_PORT from env");

    let metrics_port: u16 = env::var("METRICS_PORT")
        .expect("METRICS_PORT not set in env")
        .parse()
        .expect("Could not parse METRICS_PORT from env");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not set in env");

    let app_keys = AppKeys::from_env()?;
    let pool = connect_to_db(&database_url).await?;
    let state = AppState::new(pool.clone(), app_keys)?;
    let router = api::build_router(state.clone());

    PrometheusBuilder::new()
        .with_http_listener(([0, 0, 0, 0], metrics_port))
        .install()
        .expect("failed to install prometheus recorder");

    let mut scheduler = Scheduler::new();
    spawn_background_tasks(state, &mut scheduler);

    let app_addr = format!("0.0.0.0:{}", app_port);
    let listener = TcpListener::bind(&app_addr).await?;
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
    tracing::info!("App running on {app_addr}");

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

pub fn spawn_background_tasks(state: AppState, scheduler: &mut Scheduler) {
    scheduler.spawn_task(
        Scheduler::SECONDS_IN_DAY,
        "daily_partition",
        state.pool.clone(),
        |p| async move { link_metrics::create_partitions_task(p).await },
    );

    scheduler.spawn_task(
        15,
        "daily_metrics",
        (state.pool.clone(), state.metrics.clone()),
        |(p, m)| async move { link_metrics::process_batch_task(p, m).await },
    );

    scheduler.spawn_task(
        Scheduler::SECONDS_IN_DAY,
        "link_cleanup",
        state.pool.clone(),
        |p| async move { link_cleanup::link_cleanup_task(p).await },
    );
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
