use anyhow::Result;
use sqlx::PgPool;
use url_shorten::app::{self, AppError};

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
    let result = app.shorten_url(url, None).await;

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
    match result {
        Err(AppError::NotExists(name)) => {
            assert_eq!(name, "nonexistent");
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[sqlx::test]
async fn test_shorten_url_stress_test(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_stress_test").await;

    let mut aliases = Vec::new();
    for i in 0..100 {
        let url = format!("https://www.example{}.com", i);
        let result = app.shorten_url(&url, None).await;
        assert!(
            result.is_ok(),
            "Failed to shorten URL {}: {:?}",
            i,
            result.err()
        );
        aliases.push((result.unwrap(), url));
    }

    let alias_set: std::collections::HashSet<_> = aliases.iter().map(|(alias, _)| alias).collect();
    assert_eq!(
        alias_set.len(),
        aliases.len(),
        "All aliases should be unique"
    );

    for (alias, expected_url) in &aliases {
        let retrieved = app
            .get_url(alias)
            .await
            .expect("Should be able to retrieve URL");
        assert_eq!(
            retrieved, *expected_url,
            "Alias should resolve to correct URL"
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
            tokio::spawn(async move { app.shorten_url(&url, None).await })
        })
        .collect();

    let mut aliases = Vec::new();
    for handle in handles {
        let result = handle.await.expect("Join handle should succeed");
        let alias = result.expect("Shorten URL should succeed");
        aliases.push(alias);
    }

    // Verify all aliases are unique using HashSet for O(n) instead of O(n²)
    let alias_set: std::collections::HashSet<_> = aliases.iter().collect();
    assert_eq!(
        alias_set.len(),
        aliases.len(),
        "Concurrent inserts should produce unique aliases"
    );

    // Verify all aliases resolve to the same URL
    for alias in &aliases {
        let retrieved = app
            .get_url(alias)
            .await
            .expect("Should be able to retrieve URL");
        assert_eq!(retrieved, url, "All aliases should resolve to the same URL");
    }
}

#[sqlx::test]
async fn test_shorten_url_different_urls_produce_unique_aliases(pool: PgPool) {
    let app = setup_app(
        pool,
        "test_shorten_url_different_urls_produce_unique_aliases",
    )
    .await;

    let urls = vec![
        "https://www.example1.com",
        "https://www.example2.com",
        "https://www.example3.com",
    ];

    let mut aliases = Vec::new();
    for url in &urls {
        let result = app.shorten_url(url, None).await;
        assert!(result.is_ok(), "Failed to shorten URL: {}", url);
        aliases.push(result.unwrap());
    }

    // Verify all aliases are unique using HashSet for O(n) instead of O(n²)
    let alias_set: std::collections::HashSet<_> = aliases.iter().collect();
    assert_eq!(
        alias_set.len(),
        aliases.len(),
        "Different URLs should produce unique aliases"
    );

    // Verify all aliases resolve to correct URLs
    for (alias, url) in aliases.iter().zip(urls.iter()) {
        let retrieved = app
            .get_url(alias)
            .await
            .expect("Should be able to retrieve URL");
        assert_eq!(retrieved, *url, "Alias should resolve to correct URL");
    }
}

#[sqlx::test]
async fn test_shorten_url_allows_duplicate_urls(pool: PgPool) {
    let app = setup_app(pool, "test_shorten_url_allows_duplicate_urls").await;

    let url = "https://www.example.com";
    let alias1 = app.shorten_url(url, None).await.unwrap();
    let alias2 = app.shorten_url(url, None).await.unwrap();

    assert_ne!(
        alias1, alias2,
        "Duplicate URLs should produce different aliases"
    );

    assert_eq!(app.get_url(&alias1).await.unwrap(), url);
    assert_eq!(app.get_url(&alias2).await.unwrap(), url);
}

#[sqlx::test]
async fn test_get_last_hit(pool: PgPool) -> Result<()> {
    let app = setup_app(pool, "test_get_last_hit").await;

    let url = "https://www.example.com";
    let alias = app.shorten_url(url, None).await.unwrap();

    // just do a very rudimentery test, since I don't want to mess with sleeping in tests
    app.get_url(&alias).await?;
    let first_acces = app.get_last_hit(&alias).await?;
    app.get_url(&alias).await?;
    let last_acces = app.get_last_hit(&alias).await?;

    assert_ne!(first_acces, last_acces);

    Ok(())
}

#[sqlx::test]
async fn test_recent_hit_count(pool: PgPool) -> Result<()> {
    let app = setup_app(pool, "test_recent_hit_count").await;

    let url = "https://www.example.com";
    let alias = app.shorten_url(url, None).await.unwrap();

    for _ in 0..5 {
        app.get_url(&alias).await?;
    }

    let recent_hc = app.get_recent_hits(&alias).await?;

    assert_eq!(recent_hc, 5);

    Ok(())
}
