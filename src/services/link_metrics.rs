use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::{domain::LinkId, services::ServiceError};

#[derive(Debug, Serialize)]
pub struct LinkMetricsQuery {
    pub link_id: LinkId,
    pub total_hits: i64,
    pub hits_today: i64,
    pub hits_last_30_days: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub last_access: Option<OffsetDateTime>,
}

pub async fn list_link_metrics(
    link_id: LinkId,
    pool: &PgPool,
) -> Result<LinkMetricsQuery, ServiceError> {
    let rec = sqlx::query!(
        r#"
        SELECT
            COALESCE(SUM(hits), 0)::BIGINT AS "total_hits!",
            COALESCE(SUM(hits) FILTER (WHERE day = CURRENT_DATE), 0)::BIGINT AS "hits_today!",
            COALESCE(
                SUM(hits) FILTER (
                    WHERE day >= (CURRENT_DATE - INTERVAL '29 days')
                ),
                0
            )::BIGINT AS "hits_last_30_days!",
            MAX(last_access) AS "last_access?"
        FROM link_metrics
        WHERE link_id = $1
        "#,
        link_id
    )
    .fetch_one(pool)
    .await?;

    Ok(LinkMetricsQuery {
        link_id,
        total_hits: rec.total_hits,
        hits_today: rec.hits_today,
        hits_last_30_days: rec.hits_last_30_days,
        last_access: rec.last_access,
    })
}
