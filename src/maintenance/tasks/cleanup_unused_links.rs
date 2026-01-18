use anyhow::{Context, Result};
use sqlx::Pool;
use sqlx::Postgres;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::maintenance::cache::Cache;
use crate::maintenance::tasks::MaintenanceTask;
use crate::maintenance::usage_metrics::UsageMetrics;

/// Task to clean up unused links
///
/// Removes links that haven't been accessed in a specified period.
/// This task respects load distribution and only runs during low-traffic periods.
pub struct CleanupUnusedLinksTask {
    /// Number of days of inactivity before a link is considered unused
    days_unused: u64,
}

impl CleanupUnusedLinksTask {
    pub fn new(days_unused: u64) -> Self {
        Self { days_unused }
    }
}

impl Default for CleanupUnusedLinksTask {
    /// Default: links unused for 90 days
    fn default() -> Self {
        Self::new(90)
    }
}

#[async_trait::async_trait]
impl MaintenanceTask for CleanupUnusedLinksTask {
    fn name(&self) -> &'static str {
        "cleanup_unused_links"
    }

    async fn execute(
        &self,
        pool: &Pool<Postgres>,
        _usage_metrics: &dyn UsageMetrics,
        cache: &dyn Cache,
    ) -> Result<()> {
        tracing::info!(
            task = self.name(),
            days_unused = self.days_unused,
            "Starting cleanup of unused links"
        );

        // Calculate the cutoff timestamp
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Failed to get current time")?
            .as_secs()
            - (self.days_unused * 24 * 60 * 60);

        // Delete unused links
        // Links are considered unused if:
        // 1. They have never been accessed (last_accessed_at is NULL), AND created_at is older than cutoff
        // 2. OR last_accessed_at is older than cutoff
        let result = sqlx::query(
            r#"
            DELETE FROM links
            WHERE (
                (last_accessed_at IS NULL AND created_at < to_timestamp($1))
                OR (last_accessed_at IS NOT NULL AND last_accessed_at < to_timestamp($1))
            )
            "#,
        )
        .bind(cutoff_time as i64)
        .execute(pool)
        .await
        .context("Failed to delete unused links")?;

        let deleted_count = result.rows_affected();
        tracing::info!(
            task = self.name(),
            deleted_count = deleted_count,
            "Completed cleanup of unused links"
        );

        // Invalidate cache entries for deleted links
        // Note: This is a placeholder for Cache #14 integration
        if deleted_count > 0 {
            cache.invalidate_all().await?;
            tracing::debug!(task = self.name(), "Invalidated cache after cleanup");
        }

        Ok(())
    }

    async fn should_run(&self, usage_metrics: &dyn UsageMetrics) -> Result<bool> {
        // Only run during low-traffic periods
        let current_load = usage_metrics.get_current_load().await?;
        let is_low_traffic = usage_metrics.is_low_traffic_period().await?;

        Ok(current_load < 0.5 && is_low_traffic)
    }
}
