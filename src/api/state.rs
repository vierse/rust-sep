use std::{env, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use argon2::Argon2;
use moka::future::Cache;
use sqids::Sqids;
use sqlx::PgPool;
use time::{Date, OffsetDateTime};

use crate::{
    api::{Sessions, token::TokenSigner},
    domain::LinkAlias,
    tasks::link_metrics::LinkMetrics,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachedLinkType {
    Redirect,
    Collection,
}

#[derive(Debug, Clone)]
pub struct CachedLink {
    pub id: i64,
    pub kind: CachedLinkType,
    pub url: String,
    pub user_id: Option<i64>,
    pub last_seen: Date,
    pub password_hash: Option<String>,
    pub created_at: OffsetDateTime,
}

pub struct AppKeys {
    pub sqids_key: String,
    pub token_key: Vec<u8>,
}

impl AppKeys {
    pub fn from_env() -> Result<Self> {
        let sqids_key = env::var("SQIDS_KEY").context("SQIDS_KEY not set")?;
        let token_key = env::var("TOKEN_KEY").context("TOKEN_KEY not set")?;
        Ok(Self {
            sqids_key,
            token_key: token_key.into_bytes(),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sqids: Arc<Sqids>,
    pub metrics: Arc<LinkMetrics>,
    pub cache: Cache<LinkAlias, Option<CachedLink>>,
    pub sessions: Sessions,
    pub hasher: Arc<Argon2<'static>>,
    pub signer: Arc<TokenSigner>,
}

impl AppState {
    pub fn new(pool: PgPool, keys: AppKeys) -> Result<Self> {
        let sqids = Arc::new(
            Sqids::builder()
                .min_length(LinkAlias::MIN_RANDOM_ALIAS_LENGTH)
                .alphabet(keys.sqids_key.chars().collect())
                .build()
                .context("failed to build Sqids")?,
        );

        let cache: Cache<LinkAlias, Option<CachedLink>> = Cache::builder()
            .time_to_idle(Duration::from_secs(60 * 60 * 24))
            .max_capacity(3_000)
            .build();

        let metrics = Arc::new(LinkMetrics::new());

        Ok(Self {
            pool,
            sqids,
            metrics,
            cache,
            sessions: Sessions::default(),
            hasher: Arc::new(Argon2::default()),
            signer: Arc::new(TokenSigner::new(keys.token_key)),
        })
    }
}
