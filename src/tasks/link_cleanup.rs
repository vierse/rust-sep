use std::time::Instant;

use anyhow::Result;
use sqlx::PgPool;

const TTI_DAYS: i32 = 30;
const BATCH_SIZE: i64 = 5_000;

pub async fn link_cleanup_task(pool: PgPool) -> Result<()> {
    tracing::info!("Running link cleanup task...");

    let mut entries_deleted = 0i64;
    let start = Instant::now();
    loop {
        let row = sqlx::query!(
            r#"
            WITH expired AS (
                SELECT id
                FROM links_main
                WHERE last_seen < (CURRENT_DATE - $1::int)
                ORDER BY id
                LIMIT $2
            ),
            deleted AS (
                DELETE FROM links_main
                USING expired
                WHERE links_main.id = expired.id
                RETURNING 1
            )
            SELECT COUNT(*)::bigint AS "deleted_count!: i64"
            FROM deleted;
            "#,
            TTI_DAYS,
            BATCH_SIZE,
        )
        .fetch_one(&pool)
        .await?;

        entries_deleted += row.deleted_count;

        if row.deleted_count < BATCH_SIZE {
            break;
        }
    }

    if entries_deleted > 0 {
        tracing::info!(
            "Deleted {} entries in {} ms",
            entries_deleted,
            start.elapsed().as_millis()
        );
    } else {
        tracing::info!("Nothing to delete");
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use time::{Date, Duration as TimeDelta};

    use super::*;

    #[sqlx::test]
    async fn link_cleanup_ok(pool: PgPool) -> Result<()> {
        const LINKS_N: usize = 12_000;
        const CHUNK: usize = 5_000;

        async fn insert_link_batch(
            pool: &PgPool,
            prefix: &str,
            size: usize,
            last_seen: Date,
            chunk: usize,
        ) -> Result<()> {
            for start in (0..size).step_by(chunk) {
                let end = (start + chunk).min(size);

                let mut aliases = Vec::with_capacity(end - start);
                let mut urls = Vec::with_capacity(end - start);

                for idx in start..end {
                    aliases.push(format!("{prefix}_{idx}"));
                    urls.push(format!("https://example.com/{prefix}/{idx}"));
                }

                sqlx::query!(
                    r#"
                    INSERT INTO links_main (alias, url, last_seen)
                    SELECT a, u, $3
                    FROM UNNEST($1::text[], $2::text[]) AS t(a, u)
                    "#,
                    &aliases,
                    &urls,
                    last_seen,
                )
                .execute(pool)
                .await?;
            }
            Ok(())
        }

        let today = sqlx::query!(r#"SELECT CURRENT_DATE::date AS "today!: time::Date""#)
            .fetch_one(&pool)
            .await?
            .today;

        let cutoff = today - TimeDelta::days(TTI_DAYS as i64);
        let expired_day = cutoff - TimeDelta::days(1);

        insert_link_batch(&pool, "good", LINKS_N, today, CHUNK).await?;
        insert_link_batch(&pool, "expired", LINKS_N, expired_day, CHUNK).await?;

        link_cleanup_task(pool.clone()).await?;

        let after = sqlx::query!(
            r#"
            SELECT
            COUNT(*) FILTER (WHERE last_seen < $1)::bigint  AS "expired!: i64",
            COUNT(*) FILTER (WHERE last_seen >= $1)::bigint AS "good!: i64"
            FROM links_main
            "#,
            cutoff,
        )
        .fetch_one(&pool)
        .await?;

        assert_eq!(after.expired, 0, "Not all expired links have been deleted");
        assert_eq!(after.good, LINKS_N as i64, "Missing good links");

        Ok(())
    }
}
