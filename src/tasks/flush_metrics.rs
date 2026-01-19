use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::metrics::{Metrics, MetricsMap};

// TODO: move it into settings?
const CHUNK_SIZE: usize = 50;
const INTERVAL_S: u64 = 15;

pub async fn run(pool: PgPool, metrics: Arc<Metrics>) {
    let mut interval = tokio::time::interval(Duration::from_secs(INTERVAL_S));

    loop {
        interval.tick().await;

        let map = metrics.swap_map();
        if let Err(e) = process_batch(&pool, &map).await {
            tracing::error!(error = %e, "error when flushing metrics");
        }
    }
}

pub async fn process_batch(pool: &PgPool, map: &MetricsMap) -> Result<()> {
    if map.is_empty() {
        return Ok(());
    }

    // (link_id, hits, last_access) columns
    let mut link_id_col: Vec<i64> = Vec::with_capacity(CHUNK_SIZE);
    let mut hits_col: Vec<i64> = Vec::with_capacity(CHUNK_SIZE);
    let mut last_access_col: Vec<OffsetDateTime> = Vec::with_capacity(CHUNK_SIZE);

    let mut entries_updated = 0usize;

    for entry in map.iter() {
        let link_id = *entry.key();
        let val = entry.value();

        let hits = val.hits();
        if hits == 0 {
            continue;
        }

        let last_access = OffsetDateTime::from_unix_timestamp(val.last_access_s())
            .context("Failed to convert last access seconds (i64) back into unix timestamp")?;

        link_id_col.push(link_id);
        hits_col.push(hits);
        last_access_col.push(last_access);
        entries_updated += 1;

        // Flush once a chunk is full
        if link_id_col.len() == CHUNK_SIZE {
            flush_to_db(pool, &link_id_col, &hits_col, &last_access_col).await?;
            // Clear columns
            link_id_col.clear();
            hits_col.clear();
            last_access_col.clear();
        }
    }

    // Flush the rest
    flush_to_db(pool, &link_id_col, &hits_col, &last_access_col).await?;
    tracing::info!("Updated {} entries", entries_updated);

    Ok(())
}

async fn flush_to_db(
    pool: &PgPool,
    link_id_col: &[i64],
    hits_col: &[i64],
    last_access_col: &[OffsetDateTime],
) -> Result<()> {
    if link_id_col.is_empty() {
        return Ok(());
    }

    assert!(
        link_id_col.len() == hits_col.len() && hits_col.len() == last_access_col.len(),
        "instead {} {} {}",
        link_id_col.len(),
        hits_col.len(),
        last_access_col.len()
    );

    sqlx::query!(
        r#"
        INSERT INTO daily_hits (day, link_id, hits, last_access)
        SELECT
            CURRENT_DATE,
            t.link_id,
            t.hits,
            t.last_access
        FROM UNNEST($1::bigint[], $2::bigint[], $3::timestamptz[])
             AS t(link_id, hits, last_access)
        ON CONFLICT (day, link_id) DO UPDATE
          SET hits        = daily_hits.hits + EXCLUDED.hits,
              last_access = GREATEST(daily_hits.last_access, EXCLUDED.last_access)
        "#,
        link_id_col,
        hits_col,
        last_access_col,
    )
    .execute(pool)
    .await?;

    Ok(())
}
