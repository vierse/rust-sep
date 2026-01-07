use std::time::Duration;

use anyhow::{Context, Result, bail};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqids::Sqids;
use sqlx::{Pool, Postgres, Transaction, postgres::PgPoolOptions};
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

    #[tracing::instrument(name = "app::get_url", skip(self))]
    pub async fn get_url(&self, alias: &str) -> Result<String> {
        let rec = sqlx::query!(
            r#"
            SELECT url
            FROM links
            WHERE alias = $1
            "#,
            alias
        )
        .fetch_optional(&self.pool)
        .await
        .context("DB select query failed")?;

        match rec {
            Some(r) => Ok(r.url),
            None => bail!("This alias does not exist"),
        }
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

    Ok(AppState { pool, sqids })
}

pub async fn run(config: Settings) -> Result<()> {
    let pool = connect_to_db(config.database_url.as_str()).await?;
    let state = build_app_state(pool).await?;
    let router = api::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
