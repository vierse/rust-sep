use anyhow::Result;
use url_shorten::{app, config};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::load()?;
    let _ = app::connect_to_db(config.database_url.as_str()).await?;
    tracing::info!("DB successfully initalized");

    Ok(())
}
