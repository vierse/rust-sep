use anyhow::Result;
use async_trait::async_trait;
use url_shorten::core::BaseApp;

pub struct MockApp {
    pub url: String,
    pub alias: String,
}

#[async_trait]
impl BaseApp for MockApp {
    async fn shorten_url(&self, _url: &str) -> Result<String> {
        Ok(self.alias.clone())
    }

    async fn get_url(&self, _alias: &str) -> Result<String> {
        Ok(self.url.clone())
    }
}
