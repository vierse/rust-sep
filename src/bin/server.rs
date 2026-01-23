use anyhow::Result;

use url_shorten::{app, config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::load()?;

    app::run(config).await
}
