use anyhow::Result;
use url_shorten::core;

#[tokio::main]
async fn main() -> Result<()> {
    core::run().await
}
