use sqlx::PgPool;
use url_shorten::app;

async fn setup_app(pool: PgPool, test_name: &str) -> app::AppState {
    app::build_app_state(pool)
        .await
        .unwrap_or_else(|_| {
            panic!(
                "Failed to initialize app in test '{}'. Make sure DATABASE_URL is set and PostgreSQL is running. Start with: docker compose up postgres -d",
                test_name
            )
        })
}

#[sqlx::test]
async fn test_shorten_url_success(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_success").await;

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_ok());
    let alias = result.unwrap();
    assert!(alias.len() >= 6);

    let retrieved = app.get_url(&alias).await;
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap(), url);
}

#[sqlx::test]
async fn test_get_url_not_found(pool: PgPool) {
    let app = setup_app(pool, "test_get_url_not_found").await;

    let result = app.get_url("nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[sqlx::test]
async fn test_shorten_url_retry_on_collision(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_retry_on_collision").await;

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

#[sqlx::test]
async fn test_shorten_url_handles_concurrent_insert(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_handles_concurrent_insert").await;

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

#[sqlx::test]
async fn test_shorten_url_multiple_urls(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_multiple_urls").await;

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

    // Verify all aliases are unique
    for i in 0..aliases.len() {
        for j in (i + 1)..aliases.len() {
            assert_ne!(aliases[i], aliases[j], "Aliases should be unique");
        }
    }

    // Verify we can retrieve all URLs
    for (alias, url) in aliases.iter().zip(urls.iter()) {
        let retrieved = app.get_url(alias).await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), *url);
    }
}

#[sqlx::test]
async fn test_shorten_url_allows_duplicate_urls(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_allows_duplicate_urls").await;

    // Test that the same URL can be shortened multiple times
    // (system allows duplicates, each gets a different alias)
    let url = "https://www.example.com";
    let alias1 = app.shorten_url(url).await.unwrap();
    let alias2 = app.shorten_url(url).await.unwrap();

    // Both should succeed and be valid
    assert!(app.get_url(&alias1).await.is_ok());
    assert!(app.get_url(&alias2).await.is_ok());

    // Both should point to the same URL
    assert_eq!(app.get_url(&alias1).await.unwrap(), url);
    assert_eq!(app.get_url(&alias2).await.unwrap(), url);
}
