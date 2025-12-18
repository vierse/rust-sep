use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use rand::{Rng, distributions::Alphanumeric};
use tokio::net::TcpListener;

use crate::{
    api::build_router,
    db::{Database, SqliteDB},
};

#[async_trait]
pub trait BaseApp {
    async fn shorten_url(&self, url: &str) -> Result<String>;

    async fn get_url(&self, alias: &str) -> Result<String>;
}

#[derive(Clone)]
pub struct AppState {
    pub app: Arc<dyn BaseApp + Send + Sync>,
}

pub struct App {
    _db: Arc<dyn Database + Send + Sync>,
}

fn generate_alias() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect()
}

#[async_trait]
impl BaseApp for App {
    async fn shorten_url(&self, url: &str) -> Result<String> {
        const MAX_RETRIES: u32 = 10;

        for _ in 0..MAX_RETRIES {
            let alias = generate_alias();

            if self._db.get(&alias).await.is_ok() {
                continue;
            }

            match self._db.insert(&alias, url).await {
                Ok(()) => return Ok(alias),
                Err(e) => {
                    if self._db.get(&alias).await.is_ok() {
                        continue;
                    }
                    return Err(e);
                }
            }
        }

        anyhow::bail!(
            "Failed to generate unique alias after {} attempts",
            MAX_RETRIES
        )
    }

    async fn get_url(&self, alias: &str) -> Result<String> {
        let url = self._db.get(alias).await?;
        Ok(url)
    }
}

pub async fn run() -> Result<()> {
    let db = Arc::new(SqliteDB {});
    let app = Arc::new(App { _db: db });
    let router = build_router(AppState { app });

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, router).await.unwrap();

    Ok(())
}
