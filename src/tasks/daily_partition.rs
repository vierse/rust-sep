use std::time::Duration;

use anyhow::Result;
use chrono::{NaiveDate, TimeDelta};
use sqlx::PgPool;

pub async fn run(pool: PgPool) {
    let mut interval = tokio::time::interval(Duration::from_hours(24));

    loop {
        interval.tick().await;

        if let Err(e) = create_daily_partition(&pool).await {
            eprintln!("partition creation failed: {e:?}");
        }
    }
}

async fn create_daily_partition(pool: &PgPool) -> Result<()> {
    // TODO: time or chrono?
    let today: NaiveDate = sqlx::query_scalar("SELECT CURRENT_DATE")
        .fetch_one(pool)
        .await?;

    for offset in 0..=3 {
        let from = today + TimeDelta::days(offset);
        let to = from + TimeDelta::days(1);

        // daily_hits_YYYYMMDD
        let part_name = format!("daily_hits_{}", from.format("%Y%m%d"));
        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {part}
            PARTITION OF daily_hits
            FOR VALUES FROM ('{from}') TO ('{to}');
            "#,
            part = part_name,
            from = from.format("%Y-%m-%d"),
            to = to.format("%Y-%m-%d"),
        );

        sqlx::query(&sql).execute(pool).await?;

        tracing::info!("Created daily partition {part_name}");
    }

    Ok(())
}
