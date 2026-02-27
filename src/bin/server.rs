use anyhow::Result;
use dotenv;

use url_shorten::app;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    app::run().await
}
