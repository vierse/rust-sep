use anyhow::{Context, Result, bail};
use sqids::Sqids;
use sqlx::{PgPool, Pool, Postgres, Transaction};
use tokio::net::TcpListener;

use crate::{api, config::Settings};

#[derive(Clone)]
pub struct AppState {
    pool: Pool<Postgres>,
    sqids: Sqids,
}

impl AppState {
    #[tracing::instrument(name = "app::shorten_url", skip(self))]
    pub async fn shorten_url(&self, url: &str) -> Result<String> {
        let mut tx: Transaction<Postgres> = self.pool.begin().await?;

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
}

pub async fn build_app_state(database_url: &str) -> Result<AppState> {
    const MIN_ALIAS_LENGTH: u8 = 6;
    const ALPHABET: &str = "79Hr0JZijqWTnxhgoDEKMRpX4FNIfywG3e6LcldO5bCUYSBPa81s2QAumtzVvk";

    let sqids = Sqids::builder()
        .min_length(MIN_ALIAS_LENGTH)
        .alphabet(ALPHABET.chars().collect())
        .build()?;
    let pool = PgPool::connect(database_url).await?;
    Ok(AppState { pool, sqids })
}

pub async fn run(config: Settings) -> Result<()> {
    let state = build_app_state(config.database_url.as_str()).await?;
    let router = api::build_router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("App running on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
