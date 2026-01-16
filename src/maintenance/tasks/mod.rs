mod cleanup_unused_links;

use anyhow::Result;
use async_trait::async_trait;
use sqlx::Pool;
use sqlx::Postgres;

use crate::maintenance::cache::Cache;
use crate::maintenance::usage_metrics::UsageMetrics;

/// Trait for maintenance tasks that can be scheduled
#[async_trait]
pub trait MaintenanceTask: Send + Sync {
    /// Name of the task for logging purposes
    fn name(&self) -> &'static str;

    /// Execute the maintenance task
    async fn execute(
        &self,
        pool: &Pool<Postgres>,
        usage_metrics: &dyn UsageMetrics,
        cache: &dyn Cache,
    ) -> Result<()>;

    /// Check if this task should run based on current load
    /// Returns true if the task should execute, false otherwise
    async fn should_run(&self, usage_metrics: &dyn UsageMetrics) -> Result<bool> {
        // Default implementation: always run if load is low
        let current_load = usage_metrics.get_current_load().await?;
        Ok(current_load < 0.7) // Run if load is below 70%
    }
}

pub use cleanup_unused_links::CleanupUnusedLinksTask;
