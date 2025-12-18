    use std::sync::Arc;

    use anyhow::Result;
    use async_trait::async_trait;
    use tokio::net::TcpListener;

    use crate::{
        api::build_router,
        db::{Database, SqliteDB},
    };

    #[async_trait]
    pub trait BaseApp {
        async fn create_alias(&self, url: &str) -> Result<String>;

        async fn get_url(&self, alias: &str) -> Result<String>;
    }

    #[derive(Clone)]
    pub struct AppState {
        pub app: Arc<dyn BaseApp + Send + Sync>,
    }

    pub struct App {
        _db: Arc<dyn Database + Send + Sync>,
    }

    #[async_trait]
    impl BaseApp for App {
        async fn create_alias(&self, _url: &str) -> Result<String> {
            unimplemented!()
        }

        async fn get_url(&self, _alias: &str) -> Result<String> {
            unimplemented!()
        }
    }

    pub async fn run() -> Result<()> {
        let db = Arc::new(SqliteDB::new("sqlite:dbfile.db?mode=rwc").await?);
        let app = Arc::new(App { _db: db });
        let router = build_router(AppState { app });

        let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

        axum::serve(listener, router).await.unwrap();

        Ok(())
    }
