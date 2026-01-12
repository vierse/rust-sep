use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqids::Sqids;
use sqlx::{Pool, Postgres, Transaction, postgres::PgPoolOptions, types::time::OffsetDateTime};
use tokio::net::TcpListener;

use crate::{api, config::Settings};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
    sqids: Sqids,
}

pub enum AppError {
    AlreadyExists(String),
    NotExists(String),
    DatabaseError(sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::AlreadyExists(alias) => {
                tracing::warn!(cause = %alias, "alias already exists");
                (StatusCode::CONFLICT).into_response()
            }
            AppError::NotExists(alias) => {
                tracing::warn!(cause = %alias, "alias does not exist");
                (StatusCode::NOT_FOUND).into_response()
            }
            AppError::DatabaseError(e) => {
                tracing::error!(error = %e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR).into_response()
            }
        }
    }
}

#[derive(Debug)]
pub enum GetUrlError {
    AliasNotFount,
    /// failed to log the hit, but succeeded in getting the url
    HitLogFail(String, sqlx::Error),
    DBErr(sqlx::Error),
}

impl PartialEq for GetUrlError {
    fn eq(&self, other: &Self) -> bool {
        use GetUrlError::*;
        match (self, other) {
            (AliasNotFount, AliasNotFount) => true,
            (HitLogFail(url, _), HitLogFail(url1, _)) => url == url1,
            (DBErr(_), DBErr(_)) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for GetUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetUrlError::AliasNotFount => write!(f, "couldn't find alias in the db"),
            GetUrlError::HitLogFail(_, error) => {
                write!(
                    f,
                    "logging access of existing alias failed at the db: {error}",
                )
            }
            GetUrlError::DBErr(error) => write!(f, "failed to access the links table: {error}"),
        }
    }
}

impl std::error::Error for GetUrlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AliasNotFount => None,
            Self::HitLogFail(_, e) => Some(e),
            Self::DBErr(e) => Some(e),
        }
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl AppState {
    #[tracing::instrument(name = "app::shorten_url", skip(self))]
    pub async fn shorten_url(&self, url: &str) -> Result<String> {
        let mut tx: Transaction<Postgres> = self.pool.begin().await?;

        // Insert the url into database to get a unique id
        let rec = sqlx::query!(
            r#"
            INSERT INTO links (url)
            VALUES ($1)
            RETURNING id
            "#,
            url,
        )
        .fetch_one(&mut *tx)
        .await
        .context("DB insert url query failed")?;

        let id = rec.id as u64;

        let alias = self.sqids.encode(&[id])?;

        // Update the record with generated alias
        let updated = sqlx::query!(
            r#"
            UPDATE links
            SET alias = $1
            WHERE id = $2
            RETURNING alias
            "#,
            alias,
            rec.id
        )
        .fetch_one(&mut *tx)
        .await
        .context("DB update url query failed")?;

        tx.commit()
            .await
            .context("DB failed to commit transaction")?;

        let alias = updated.alias.context("Alias was not set after update")?;

        Ok(alias)
    }

    /// queries a url, directly updating the hitcount
    /// # Errors
    /// fails if the requested alias does not exist in the database
    #[tracing::instrument(name = "app::get_url", skip(self))]
    pub async fn get_url(&self, alias: &str) -> std::result::Result<String, GetUrlError> {
        let link = sqlx::query!(
            r#"
            UPDATE links
            SET hitcount = hitcount + 1, last_access = now()
            WHERE alias = $1
            RETURNING url, id
            "#,
            alias
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(GetUrlError::DBErr)?;

        let Some(link) = link else {
            return Err(GetUrlError::AliasNotFount);
        };

        if let Err(db_err) = sqlx::query!(
            r#"
            INSERT INTO recent_hits (link_id)
            VALUES ($1)
            "#,
            link.id
        )
        .execute(&self.pool)
        .await
        {
            return Err(GetUrlError::HitLogFail(link.url, db_err));
        }

        Ok(link.url)
    }

    #[tracing::instrument(name = "app::save_named_url", skip(self))]
    pub async fn save_named_url(&self, alias: &str, url: &str) -> Result<(), AppError> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO links (alias, url)
            VALUES ($1, $2)
            ON CONFLICT (alias) DO NOTHING
            RETURNING id
            "#,
            alias,
            url
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?;

        match rec {
            Some(_) => Ok(()),
            None => Err(AppError::AlreadyExists(alias.to_string())),
        }
    }

    #[tracing::instrument(name = "app::get_recent_hits", skip(self))]
    pub async fn get_recent_hits(&self, alias: &str) -> Result<u64> {
        let rec = sqlx::query!(
            r#"
            SELECT COUNT(*)
            FROM recent_hits
            WHERE link_id IN (
                SELECT id
                FROM links
                WHERE alias = $1
            )
            "#,
            alias
        )
        .fetch_one(&self.pool)
        .await?;

        rec.count
            .ok_or(anyhow!(
                "fetching the number of recent hits for link {alias} returned None"
            ))
            .map(|c| c as u64)
    }

    #[tracing::instrument(name = "app::get_last_hit", skip(self))]
    pub async fn get_last_hit(&self, alias: &str) -> Result<OffsetDateTime> {
        let rec = sqlx::query!(
            r#"
            SELECT last_access
            FROM links
            WHERE alias = $1
            "#,
            alias
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(rec.last_access)
    }
}

pub async fn connect_to_db(database_url: &str) -> Result<Pool<Postgres>> {
    // Connect to database
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("Failed to connect to database")?;

    // Run SQL migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("SQL migrations failed")?;

    Ok(pool)
}

pub async fn build_app_state(pool: Pool<Postgres>) -> Result<AppState> {
    const MIN_ALIAS_LENGTH: u8 = 6;
    // Shuffled alphabet for Sqids to generate ids from
    const ALPHABET: &str = "79Hr0JZijqWTnxhgoDEKMRpX4FNIfywG3e6LcldO5bCUYSBPa81s2QAumtzVvk";

    // Initialize Sqids generator
    let sqids = Sqids::builder()
        .min_length(MIN_ALIAS_LENGTH)
        .alphabet(ALPHABET.chars().collect())
        .build()?;

    let _ = create_daily_partition(&pool).await;

    Ok(AppState { pool, sqids })
}

/// tasks that will roughyl run every hour, tending to run slightly less often
pub async fn maintenance(pool: Pool<Postgres>) {
    let span = tracing::span!(tracing::Level::INFO, "hourly_maintenance");
    let _guard = span.enter();
    loop {
        // sleep for an hour
        tokio::time::sleep(Duration::from_secs(60 * 60)).await;

        if let Err(e) = create_daily_partition(&pool).await {
            tracing::error!("failed to create new partition of recent_hits: {e}");
        }

        if let Err(e) = clear_old_hits(&pool).await {
            tracing::error!("failed to clear older hits: {e}");
        } else {
            tracing::info!("cleared hits old than TODO");
        }
    }
}

async fn create_daily_partition(pool: &Pool<Postgres>) -> Result<()> {
    let table_name_format = time::macros::format_description!("recent_hits_[year]_[month]_[day]");

    let today = time::OffsetDateTime::now_utc().date();
    let tomorrow = today.next_day().expect("today is MAX day");

    let partition_name = today
        .format(&table_name_format)
        .expect("formatting partition table name for recent hits failed");

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {partition_name} PARTITION OF recent_hits
        FOR VALUES FROM ('{}') TO ('{}')
        "#,
        today
            .midnight()
            .format(&time::format_description::well_known::Iso8601::DATE_TIME)?,
        tomorrow
            .midnight()
            .format(&time::format_description::well_known::Iso8601::DATE_TIME)?
    ))
    .execute(pool)
    .await?;

    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {partition_name} PARTITION OF recent_hits
        FOR VALUES FROM ('{}') TO ('{}')
        "#,
        tomorrow
            .midnight()
            .format(&time::format_description::well_known::Iso8601::DATE_TIME)?,
        tomorrow
            .next_day()
            .expect("tomorrow is MAX day")
            .midnight()
            .format(&time::format_description::well_known::Iso8601::DATE_TIME)?
    ))
    .execute(pool)
    .await?;

    Ok(())
}

async fn clear_old_hits(pool: &Pool<Postgres>) -> Result<()> {
    let table_name_format = time::macros::format_description!("recent_hits_[year]_[month]_[day]");
    let month_ago = time::OffsetDateTime::now_utc()
        .date()
        .saturating_sub(time::Duration::days(31));

    let partition_name = month_ago
        .format(&table_name_format)
        .expect("formatting partition table name for recent hits failed");

    sqlx::query(&format!(
        r#"
        DROP TABLE IF NOT EXISTS {partition_name}
        "#,
    ))
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn run(config: Settings) -> Result<()> {
    let pool = connect_to_db(config.database_url.as_str()).await?;
    let state = Arc::new(build_app_state(pool.clone()).await?);
    let router = api::build_router(state.clone());

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");

    tokio::task::spawn(maintenance(pool));

    axum::serve(listener, router).await?;

    Ok(())
}
