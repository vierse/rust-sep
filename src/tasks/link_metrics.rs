use std::{
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::Instant,
};

use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use sqlx::PgPool;
use time::{
    Date, Duration as TimeDelta, OffsetDateTime, format_description::StaticFormatDescription,
    macros::format_description,
};

pub struct LinkMetricsData {
    hits: AtomicI64,
    last_access_s: AtomicI64,
}

impl LinkMetricsData {
    pub fn new(last_access_s: i64) -> Self {
        Self {
            hits: AtomicI64::new(1),
            last_access_s: AtomicI64::new(last_access_s),
        }
    }

    pub fn hits(&self) -> i64 {
        self.hits.load(Ordering::Relaxed)
    }

    pub fn last_access_s(&self) -> i64 {
        self.last_access_s.load(Ordering::Relaxed)
    }
}

pub type LinkMetricsMap = DashMap<i64, LinkMetricsData>;

pub struct LinkMetrics {
    current: ArcSwap<LinkMetricsMap>,
}

impl LinkMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_hit(&self, link_id: i64) {
        let now_s = OffsetDateTime::now_utc().unix_timestamp();

        let map = self.current.load();
        let val = map.entry(link_id).or_insert(LinkMetricsData::new(now_s));

        // increment hitcount
        val.hits.fetch_add(1, Ordering::Relaxed);

        // update last access timestamp
        let mut last_access_s = val.last_access_s.load(Ordering::Relaxed);
        while now_s > last_access_s {
            match val.last_access_s.compare_exchange_weak(
                last_access_s,
                now_s,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(next) => last_access_s = next,
            }
        }
    }

    pub fn swap_map(&self) -> Arc<LinkMetricsMap> {
        self.current.swap(Arc::new(DashMap::new()))
    }
}

impl Default for LinkMetrics {
    fn default() -> Self {
        Self {
            current: ArcSwap::from_pointee(DashMap::new()),
        }
    }
}

pub async fn process_batch_task(pool: PgPool, metrics: Arc<LinkMetrics>) -> Result<()> {
    const CHUNK_SIZE: usize = 500;

    let map: Arc<LinkMetricsMap> = metrics.swap_map();

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

static PART_NAME_DATE_FD: StaticFormatDescription = format_description!("[year][month][day]");
static ISO_DATE_FD: StaticFormatDescription = format_description!("[year]-[month]-[day]");

pub async fn create_partitions_task(pool: PgPool) -> Result<()> {
    tracing::info!("Creating daily metrics partitions...");

    let today: Date = sqlx::query_scalar("SELECT CURRENT_DATE")
        .fetch_one(&pool)
        .await?;

    // Create partitions for 4 days
    for offset in 0..=3 {
        let start = today + TimeDelta::days(offset);
        let end = start + TimeDelta::days(1);

        let iso_start = start.format(&ISO_DATE_FD)?;
        let iso_end = end.format(&ISO_DATE_FD)?;

        // daily_metrics_YYYYMMDD
        let part_name = format!("daily_metrics_{}", start.format(&PART_NAME_DATE_FD)?);
        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {part}
            PARTITION OF daily_metrics
            FOR VALUES FROM ('{from}') TO ('{to}');
            "#,
            part = part_name,
            from = iso_start,
            to = iso_end,
        );

        sqlx::query(&sql).execute(&pool).await?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn date_formatting() {
        let date = time::macros::date!(2026 - 01 - 19);
        assert_eq!(date.format(&PART_NAME_DATE_FD).unwrap(), "20260119");
        assert_eq!(date.format(&ISO_DATE_FD).unwrap(), "2026-01-19");
    }
}
