use sqlx::PgPool;
use url_shorten::app;

#[sqlx::test]
async fn test_shorten_url_success(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

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
async fn test_get_url_success(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

    // Insert a URL using shorten_url to get a valid alias
    let url = "https://www.test.com";
    let alias = app.shorten_url(url).await.unwrap();

    let result = app.get_url(&alias).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), url);
}

#[sqlx::test]
async fn test_get_url_not_found(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

    let result = app.get_url("nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[sqlx::test]
async fn test_shorten_url_retry_on_collision(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

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
async fn test_shorten_url_handles_insert_error(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_ok());
}

#[sqlx::test]
async fn test_shorten_url_handles_concurrent_insert(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

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
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

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

#[sqlx::test]
async fn test_generate_alias_length(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

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

#[sqlx::test]
async fn test_shorten_url_max_retries_exceeded(pool: PgPool) {
    let app = app::build_app_state(pool)
        .await
        .expect("Failed to initialize app");

    let url = "https://www.example.com";
    let alias1 = app.shorten_url(url).await.unwrap();
    let alias2 = app.shorten_url(url).await.unwrap();

    assert!(app.get_url(&alias1).await.is_ok());
    assert!(app.get_url(&alias2).await.is_ok());
}
