use anyhow::Result;
use sqlx::PgPool;
use time::{
    Date, Duration as TimeDelta, format_description::StaticFormatDescription,
    macros::format_description,
};

static PART_NAME_DATE_FD: StaticFormatDescription = format_description!("[year][month][day]");
static ISO_DATE_FD: StaticFormatDescription = format_description!("[year]-[month]-[day]");

pub async fn create_daily_partitions(pool: PgPool) -> Result<()> {
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
