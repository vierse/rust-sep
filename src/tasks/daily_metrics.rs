use std::{sync::Arc, time::Instant};

use anyhow::{Context, Result};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::metrics::{Metrics, MetricsMap};

// TODO: move it into settings?
const CHUNK_SIZE: usize = 500;

pub async fn process_daily_metrics(pool: PgPool, metrics: Arc<Metrics>) -> Result<()> {
    let map: Arc<MetricsMap> = metrics.swap_map();

    if map.is_empty() {
        return Ok(());
    }

    let start = Instant::now();

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
            flush_to_db(&pool, &link_id_col, &hits_col, &last_access_col).await?;
            // Clear columns
            link_id_col.clear();
            hits_col.clear();
            last_access_col.clear();
        }
    }

    // Flush the rest
    flush_to_db(&pool, &link_id_col, &hits_col, &last_access_col).await?;

    let elapsed_ms = start.elapsed().as_millis();
    tracing::info!("Updated {} entries in {} ms", entries_updated, elapsed_ms);

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

    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"
        INSERT INTO daily_metrics (day, link_id, hits, last_access)
        SELECT
            CURRENT_DATE,
            t.link_id,
            t.hits,
            t.last_access
        FROM UNNEST($1::bigint[], $2::bigint[], $3::timestamptz[])
            AS t(link_id, hits, last_access)
        ON CONFLICT (day, link_id) DO UPDATE
          SET hits = daily_metrics.hits + EXCLUDED.hits,
              last_access = GREATEST(daily_metrics.last_access, EXCLUDED.last_access)
        "#,
        link_id_col,
        hits_col,
        last_access_col,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"
        WITH ids AS (
          SELECT link_id
          FROM UNNEST($1::bigint[]) AS t(link_id)
        )
        UPDATE links_main
        SET last_seen = CURRENT_DATE
        FROM ids
        WHERE links_main.id = ids.link_id
          AND links_main.last_seen < CURRENT_DATE
        "#,
        link_id_col,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}
