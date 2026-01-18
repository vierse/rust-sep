use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Trait for usage metrics tracking
///
/// This is designed to integrate with Usage Metrics #15.
/// Provides load distribution information for maintenance task scheduling.
#[async_trait]
pub trait UsageMetrics: Send + Sync {
    /// Get the current system load (0.0 to 1.0)
    /// 0.0 = no load, 1.0 = maximum load
    async fn get_current_load(&self) -> Result<f64>;

    /// Check if we're currently in a low-traffic period
    async fn is_low_traffic_period(&self) -> Result<bool>;

    /// Record a link access for metrics tracking
    async fn record_access(&self, alias: &str) -> Result<()>;
}

/// Default implementation of UsageMetrics
///
/// Tracks request rate and calculates load based on recent activity.
pub struct DefaultUsageMetrics {
    pool: Pool<Postgres>,
    /// Recent request timestamps (last N requests)
    recent_requests: Arc<RwLock<Vec<SystemTime>>>,
    /// Window size for calculating load
    window_size: Duration,
    /// Maximum requests per window to be considered "low load"
    max_requests_per_window: usize,
}

impl DefaultUsageMetrics {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            recent_requests: Arc::new(RwLock::new(Vec::new())),
            window_size: Duration::from_secs(60), // 1 minute window
            max_requests_per_window: 100,         // 100 requests per minute = low load
        }
    }

    /// Clean up old request timestamps outside the window
    async fn cleanup_old_requests(&self) {
        let now = SystemTime::now();
        let mut requests = self.recent_requests.write().await;
        requests.retain(|&timestamp| {
            now.duration_since(timestamp)
                .map(|d| d < self.window_size)
                .unwrap_or(false)
        });
    }

    /// Determine if current hour is typically low-traffic
    /// Simple heuristic: 2 AM - 6 AM UTC is considered low-traffic
    async fn is_low_traffic_hour(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Get UTC hour (simplified - in production, use proper timezone handling)
        let hours_since_epoch = now / 3600;
        let hour_of_day = (hours_since_epoch % 24) as u8;

        // 2 AM - 6 AM UTC is low-traffic period
        (2..6).contains(&hour_of_day)
    }
}

#[async_trait]
impl UsageMetrics for DefaultUsageMetrics {
    async fn get_current_load(&self) -> Result<f64> {
        self.cleanup_old_requests().await;

        let requests = self.recent_requests.read().await;
        let request_count = requests.len();

        // Calculate load as ratio of current requests to max requests per window
        let load = (request_count as f64 / self.max_requests_per_window as f64).min(1.0);

        Ok(load)
    }

    async fn is_low_traffic_period(&self) -> Result<bool> {
        let current_load = self.get_current_load().await?;
        let is_low_traffic_hour = self.is_low_traffic_hour().await;

        // Low traffic if both conditions are met:
        // 1. Current load is low (< 0.3)
        // 2. It's a typically low-traffic hour
        Ok(current_load < 0.3 && is_low_traffic_hour)
    }

    async fn record_access(&self, alias: &str) -> Result<()> {
        // Record timestamp for load calculation
        {
            let mut requests = self.recent_requests.write().await;
            requests.push(SystemTime::now());
        }

        // Update database with last_accessed_at timestamp
        sqlx::query(
            r#"
            UPDATE links
            SET last_accessed_at = now()
            WHERE alias = $1
            "#,
        )
        .bind(alias)
        .execute(&self.pool)
        .await
        .context("Failed to update last_accessed_at")?;

        Ok(())
    }
}
