use std::sync::Arc;

use anyhow::Result;

use crate::app::Diag;

pub async fn print_diagnostics_task(diag: Arc<Diag>) -> Result<()> {
    let (cache_hits, cache_misses) = diag.snapshot();
    let total = cache_hits + cache_misses;
    let eff = if total == 0 {
        0.0
    } else {
        cache_hits as f64 / total as f64
    };
    tracing::info!(
        "eff={}, cache_hits={}, cache_misses={}",
        eff,
        cache_hits,
        cache_misses
    );
    Ok(())
}
