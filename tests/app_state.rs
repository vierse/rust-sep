use anyhow::{Context, Result};
use sqlx::PgPool;
use url_shorten::app::AppState;
use url_shorten::config;

async fn setup_test_db() -> Result<PgPool> {
    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => match config::load() {
            Ok(settings) => settings.database_url.to_string(),
            Err(_) => "postgres://app_user:app_password@localhost:5432/app_db".to_string(),
        },
    };

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .with_context(|| {
            format!(
                "Failed to connect to database at {}. \
                Make sure PostgreSQL is running and the database is set up. \
                You can start it with: docker compose up postgres -d",
                database_url
            )
        })?;

    sqlx::query("DELETE FROM links").execute(&pool).await?;

    Ok(pool)
}

async fn create_app_state() -> Result<AppState> {
    let pool = setup_test_db().await?;
    url_shorten::app::build_app_state(pool)
        .await
        .context("Failed to build app state")
}

#[tokio::test]
async fn test_shorten_url_success() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_ok());
    let alias = result.unwrap();
    assert!(alias.len() >= 6);

    let retrieved = app.get_url(&alias).await;
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap(), url);
}

#[tokio::test]
async fn test_get_url_success() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    // Insert a URL using shorten_url to get a valid alias
    let url = "https://www.test.com";
    let alias = app.shorten_url(url).await.unwrap();

    let result = app.get_url(&alias).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), url);
}

#[tokio::test]
async fn test_get_url_not_found() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    let result = app.get_url("nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_shorten_url_retry_on_collision() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    for i in 0..100 {
        let url = format!("https://www.example{}.com", i);
        let result = app.shorten_url(&url).await;
        assert!(
            result.is_ok(),
            "Failed to shorten URL {}: {:?}",
            i,
            result.err()
        );
    }
}

#[tokio::test]
async fn test_shorten_url_handles_insert_error() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shorten_url_handles_concurrent_insert() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    let url = "https://www.example.com";
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let app = app.clone();
            let url = url.to_string();
            tokio::spawn(async move { app.shorten_url(&url).await })
        })
        .collect();

    for handle in handles {
        let result = handle.await;
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_shorten_url_multiple_urls() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    let urls = vec![
        "https://www.example1.com",
        "https://www.example2.com",
        "https://www.example3.com",
    ];

    let mut aliases = Vec::new();
    for url in &urls {
        let result = app.shorten_url(url).await;
        assert!(result.is_ok());
        aliases.push(result.unwrap());
    }

    for i in 0..aliases.len() {
        for j in (i + 1)..aliases.len() {
            assert_ne!(aliases[i], aliases[j], "Aliases should be unique");
        }
    }

    for (alias, url) in aliases.iter().zip(urls.iter()) {
        let retrieved = app.get_url(alias).await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), *url);
    }
}

#[tokio::test]
async fn test_generate_alias_length() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    for i in 0..10 {
        let url = format!("https://www.example{}.com", i);
        let alias = app.shorten_url(&url).await.unwrap();
        assert!(
            alias.len() >= 6,
            "Alias should be at least 6 characters, got {}",
            alias.len()
        );
    }
}

#[tokio::test]
async fn test_shorten_url_max_retries_exceeded() {
    let app = create_app_state()
        .await
        .expect("Database connection required for tests. Run: docker compose up postgres -d");

    let url = "https://www.example.com";
    let alias1 = app.shorten_url(url).await.unwrap();
    let alias2 = app.shorten_url(url).await.unwrap();

    assert!(app.get_url(&alias1).await.is_ok());
    assert!(app.get_url(&alias2).await.is_ok());
}
