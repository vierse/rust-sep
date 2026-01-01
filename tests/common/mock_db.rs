use anyhow::Result;
use async_trait::async_trait;
use url_shorten::db::Database;

pub struct MockDB {
    alias: String,
}

#[async_trait]
impl Database for MockDB {
    async fn insert(&self, _alias: &str, _url: &str) -> Result<()> {
        Ok(())
    }

    async fn get(&self, _alias: &str) -> Result<String> {
        Ok(self.alias.clone())
    }

    async fn exists(&self, _alias: &str) -> Result<bool> {
        Ok(true)
    }

    async fn remove(&self, _alias: &str) -> Result<bool> {
        Ok(true)
    }
}
