use std::time::Duration;

use tokio::{
    task::JoinSet,
    time::{self, Instant},
};
use tokio_util::sync::CancellationToken;

pub type Task = (&'static str, anyhow::Result<()>);

pub struct Scheduler {
    cancel_token: CancellationToken,
    tasks: JoinSet<Task>,
}

impl Scheduler {
    pub const SECONDS_IN_DAY: u64 = 24 * 60 * 60;

    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a background task, immediately running it at provided interval
    pub fn spawn_task<P, F, Fut>(
        &mut self,
        interval_s: u64,
        name: &'static str,
        params: P,
        mut task: F,
    ) where
        P: Clone + Send + Sync + 'static,
        F: FnMut(P) -> Fut + Send + 'static,
        Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        let cancel = self.cancel_token.clone();
        self.tasks.spawn(async move {
            let mut interval = time::interval(Duration::from_secs(interval_s));

            loop {
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    _ = interval.tick() => {
                        if let Err(e) = task(params.clone()).await {
                            tracing::error!(error = %e, "Task {name} failed");
                            return (name, Err(e));
                        }
                    }
                }
            }

            (name, Ok(()))
        });
    }

    /// Shutdowns the scheduler, cancelling all tasks and waiting on them to finish within provided timeout in seconds
    ///
    /// Note: upon timeout, remaining tasks are NOT aborted nor drained (currently assuming the app quits afterwards)
    pub async fn shutdown(mut self, timeout_s: u64) {
        self.cancel_token.cancel();

        let timeout = Instant::now() + Duration::from_secs(timeout_s);

        // drain tasks until timeout
        while !self.tasks.is_empty() {
            let remaining = timeout.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }

            match time::timeout(remaining, self.tasks.join_next()).await {
                Ok(Some(join_result)) => match join_result {
                    Ok((name, Ok(()))) => {
                        tracing::info!("Task {name} finished successfully");
                    }
                    Ok((name, Err(e))) => {
                        tracing::error!(error = %e, "Task {name} error");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Task join error");
                    }
                },
                // no tasks left
                Ok(None) => break,
                Err(_) => {
                    tracing::error!("Scheduler timed out when shutting down");
                }
            }
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            cancel_token: CancellationToken::new(),
            tasks: JoinSet::new(),
        }
    }
}
