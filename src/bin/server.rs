use anyhow::Result;
use url_shorten::core;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    core::run().await
}
