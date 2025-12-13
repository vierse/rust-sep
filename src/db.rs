use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Database {
    async fn insert(&self, alias: &str, url: &str) -> Result<()>;

    async fn get(&self, alias: &str) -> Result<String>;
}

pub struct SqliteDB {}

#[async_trait]
impl Database for SqliteDB {
    async fn insert(&self, _alias: &str, _url: &str) -> Result<()> {
        unimplemented!()
    }

    async fn get(&self, _alias: &str) -> Result<String> {
        unimplemented!()
    }
}
