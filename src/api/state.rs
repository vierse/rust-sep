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
    services::CollectionItem,
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
    pub target_url: Option<String>,
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
    pub link_cache: Cache<LinkAlias, CachedLink>,
    pub link_cache_neg: Cache<LinkAlias, ()>,
    pub coll_cache: Cache<i64, Vec<CollectionItem>>,
    pub sessions: Sessions,
    pub hasher: Arc<Argon2<'static>>,
    pub signer: Arc<TokenSigner>,
}

impl AppState {
    const LINK_CACHE_CAP: u64 = 3_000;
    const LINK_CACHE_TTI: u64 = 24 * 60 * 60;
    const LINK_CACHE_NEG_CAP: u64 = 300;
    const LINK_CACHE_NEG_TTL: u64 = 60;
    const COLL_CACHE_CAP: u64 = 500;
    const COLL_CACHE_TTI: u64 = 24 * 60 * 60;

    pub fn new(pool: PgPool, keys: AppKeys) -> Result<Self> {
        let sqids = Arc::new(
            Sqids::builder()
                .min_length(LinkAlias::MIN_RANDOM_ALIAS_LENGTH)
                .alphabet(keys.sqids_key.chars().collect())
                .build()
                .context("failed to build Sqids")?,
        );

        let link_cache: Cache<LinkAlias, CachedLink> = Cache::builder()
            .time_to_idle(Duration::from_secs(Self::LINK_CACHE_TTI))
            .max_capacity(Self::LINK_CACHE_CAP)
            .build();

        let link_cache_neg: Cache<LinkAlias, ()> = Cache::builder()
            .time_to_live(Duration::from_secs(Self::LINK_CACHE_NEG_TTL))
            .max_capacity(Self::LINK_CACHE_NEG_CAP)
            .build();

        let coll_cache: Cache<i64, Vec<CollectionItem>> = Cache::builder()
            .time_to_idle(Duration::from_secs(Self::COLL_CACHE_TTI))
            .max_capacity(Self::COLL_CACHE_CAP)
            .build();

        let metrics = Arc::new(LinkMetrics::new());

        Ok(Self {
            pool,
            sqids,
            metrics,
            link_cache,
            link_cache_neg,
            coll_cache,
            sessions: Sessions::default(),
            hasher: Arc::new(Argon2::default()),
            signer: Arc::new(TokenSigner::new(keys.token_key)),
        })
    }
}
