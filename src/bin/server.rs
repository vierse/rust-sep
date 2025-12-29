use anyhow::Result;
use url_shorten::{config, core};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::load()?;

    core::run(config).await
}
