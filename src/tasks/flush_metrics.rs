use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, QueryBuilder};
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
        if let Err(_) = flush_metrics(&pool, &map).await {
            tracing::error!("error when flushing metrics");
        }
    }
}

async fn flush_metrics(pool: &PgPool, map: &MetricsMap) -> Result<()> {
    if map.is_empty() {
        return Ok(());
    }

    let mut rows: Vec<(i64, i64, OffsetDateTime)> = Vec::with_capacity(map.len());

    for entry in map.iter() {
        let key = entry.key();
        let val = entry.value();

        let hits = val.hits();
        if hits == 0 {
            continue;
        }

        let last_access = OffsetDateTime::from_unix_timestamp(val.last_access_s())
            .context("Failed to convert last access seconds (i64) back into unix timestamp")?;

        rows.push((*key, hits, last_access));
    }

    if rows.is_empty() {
        return Ok(());
    }

    for chunk in rows.chunks(CHUNK_SIZE) {
        db_query(pool, chunk).await?;
    }

    tracing::info!("Updated {} entries", rows.len());

    Ok(())
}

pub async fn db_query(pool: &PgPool, chunk: &[(i64, i64, OffsetDateTime)]) -> Result<()> {
    if chunk.is_empty() {
        return Ok(());
    }

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
        INSERT INTO daily_hits (day, link_id, hits, last_access)
        "#,
    );

    qb.push_values(chunk, |mut b, (link_id, hits, last_access)| {
        b.push("CURRENT_DATE")
            .push_bind(link_id)
            .push_bind(hits)
            .push_bind(last_access);
    });

    qb.push(
        r#"
        ON CONFLICT (day, link_id)
        DO UPDATE SET
            hits = daily_hits.hits + EXCLUDED.hits,
            last_access = GREATEST(daily_hits.last_access, EXCLUDED.last_access);
        "#,
    );

    qb.build().execute(pool).await?;

    Ok(())
}
