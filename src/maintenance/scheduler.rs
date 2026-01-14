use anyhow::Result;
use sqlx::Pool;
use sqlx::Postgres;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::maintenance::tasks::MaintenanceTask;
use crate::maintenance::usage_metrics::UsageMetrics;
use crate::maintenance::cache::Cache;

/// Scheduler for maintenance tasks
/// 
/// Runs maintenance tasks periodically, respecting load distribution.
/// Tasks are only executed during low-traffic periods to avoid impacting performance.
pub struct MaintenanceScheduler {
    pool: Pool<Postgres>,
    usage_metrics: Arc<dyn UsageMetrics>,
    cache: Arc<dyn Cache>,
    tasks: Vec<Arc<dyn MaintenanceTask>>,
    /// Interval between scheduler checks
    check_interval: Duration,
}

impl MaintenanceScheduler {
    pub fn new(
        pool: Pool<Postgres>,
        usage_metrics: Arc<dyn UsageMetrics>,
        cache: Arc<dyn Cache>,
    ) -> Self {
        Self {
            pool,
            usage_metrics,
            cache,
            tasks: Vec::new(),
            check_interval: Duration::from_secs(300), // Check every 5 minutes
        }
    }

    /// Add a maintenance task to the scheduler
    pub fn add_task(&mut self, task: Arc<dyn MaintenanceTask>) {
        self.tasks.push(task);
    }

    /// Set the check interval for the scheduler
    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// Start the scheduler
    /// 
    /// This will run indefinitely, checking and executing tasks based on load.
    pub async fn start(&self) -> Result<()> {
        let mut interval_timer = interval(self.check_interval);

        tracing::info!(
            task_count = self.tasks.len(),
            check_interval_secs = self.check_interval.as_secs(),
            "Starting maintenance scheduler"
        );

        loop {
            interval_timer.tick().await;

            // Check each task and execute if conditions are met
            for task in &self.tasks {
                match task.should_run(self.usage_metrics.as_ref()).await {
                    Ok(true) => {
                        tracing::debug!(
                            task = task.name(),
                            "Task conditions met, executing"
                        );

                        match task.execute(&self.pool, self.usage_metrics.as_ref(), self.cache.as_ref()).await {
                            Ok(()) => {
                                tracing::info!(
                                    task = task.name(),
                                    "Task completed successfully"
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    task = task.name(),
                                    error = %e,
                                    "Task execution failed"
                                );
                            }
                        }
                    }
                    Ok(false) => {
                        tracing::debug!(
                            task = task.name(),
                            "Task conditions not met, skipping"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            task = task.name(),
                            error = %e,
                            "Failed to check if task should run"
                        );
                    }
                }
            }
        }
    }
}

