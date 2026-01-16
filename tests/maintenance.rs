use sqlx::PgPool;
use std::sync::Arc;
use url_shorten::maintenance::{
    Cache, DefaultUsageMetrics, MaintenanceScheduler, MaintenanceTask, NoOpCache, UsageMetrics,
    tasks::CleanupUnusedLinksTask,
};

#[tokio::test]
async fn test_noop_cache_invalidate() {
    let cache = NoOpCache;

    // Should not panic
    assert!(cache.invalidate("test_key").await.is_ok());
    assert!(cache.invalidate_all().await.is_ok());
}

#[sqlx::test]
async fn test_default_usage_metrics_load_calculation(pool: PgPool) {
    let metrics = DefaultUsageMetrics::new(pool);

    // Initially, load should be low
    let initial_load = metrics.get_current_load().await.unwrap();
    assert!(initial_load >= 0.0 && initial_load <= 1.0);

    // Record some accesses to increase load
    for _ in 0..50 {
        metrics.record_access("test_alias").await.unwrap();
    }

    // Load should have increased
    let new_load = metrics.get_current_load().await.unwrap();
    assert!(new_load >= initial_load);
}

#[sqlx::test]
async fn test_default_usage_metrics_record_access(pool: PgPool) {
    let metrics = DefaultUsageMetrics::new(pool.clone());

    // Create a test link first
    let alias = "test_alias_123";
    sqlx::query(
        r#"
        INSERT INTO links (alias, url)
        VALUES ($1, $2)
        ON CONFLICT (alias) DO NOTHING
        "#,
    )
    .bind(alias)
    .bind("https://example.com")
    .execute(&pool)
    .await
    .unwrap();

    // Record access
    assert!(metrics.record_access(alias).await.is_ok());

    // Verify last_accessed_at was updated by checking count
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM links WHERE alias = $1 AND last_accessed_at IS NOT NULL
        "#,
    )
    .bind(alias)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count.0, 1, "last_accessed_at should have been set");

    // Cleanup
    sqlx::query("DELETE FROM links WHERE alias = $1")
        .bind(alias)
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_cleanup_unused_links_task_execute(pool: PgPool) {
    let task = CleanupUnusedLinksTask::new(0); // 0 days for testing
    let cache: Arc<dyn Cache> = Arc::new(NoOpCache);
    let metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));

    // Test 1: Create an old link that was never accessed (NULL last_accessed_at)
    let never_accessed_alias = "never_accessed_test";
    sqlx::query(
        r#"
        INSERT INTO links (alias, url, created_at)
        VALUES ($1, $2, now() - interval '1 day')
        "#,
    )
    .bind(never_accessed_alias)
    .bind("https://never-accessed.example.com")
    .execute(&pool)
    .await
    .unwrap();

    // Test 2: Create a link that was accessed but is now old
    let old_accessed_alias = "old_accessed_test";
    sqlx::query(
        r#"
        INSERT INTO links (alias, url, created_at, last_accessed_at)
        VALUES ($1, $2, now() - interval '2 days', now() - interval '1 day')
        "#,
    )
    .bind(old_accessed_alias)
    .bind("https://old-accessed.example.com")
    .execute(&pool)
    .await
    .unwrap();

    // Test 3: Create a recent link
    let recent_alias = "recent_link_test";
    sqlx::query(
        r#"
        INSERT INTO links (alias, url, created_at, last_accessed_at)
        VALUES ($1, $2, now(), now())
        "#,
    )
    .bind(recent_alias)
    .bind("https://recent.example.com")
    .execute(&pool)
    .await
    .unwrap();

    // Execute cleanup task
    let result = task.execute(&pool, metrics.as_ref(), cache.as_ref()).await;
    assert!(result.is_ok());

    // Verify old links were deleted (both never-accessed and old-accessed)
    let never_accessed_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1")
            .bind(never_accessed_alias)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        never_accessed_count.0, 0,
        "Never-accessed old link should have been deleted"
    );

    let old_accessed_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1")
        .bind(old_accessed_alias)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        old_accessed_count.0, 0,
        "Old accessed link should have been deleted"
    );

    // Verify recent link still exists
    let recent_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1")
        .bind(recent_alias)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(recent_count.0, 1, "Recent link should still exist");

    // Cleanup
    sqlx::query("DELETE FROM links WHERE alias = $1 OR alias = $2 OR alias = $3")
        .bind(never_accessed_alias)
        .bind(old_accessed_alias)
        .bind(recent_alias)
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_maintenance_scheduler_add_task(pool: PgPool) {
    let usage_metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));
    let cache: Arc<dyn Cache> = Arc::new(NoOpCache);

    let mut scheduler = MaintenanceScheduler::new(pool, usage_metrics, cache);

    // Add tasks - should not panic
    let task: Arc<dyn MaintenanceTask> = Arc::new(CleanupUnusedLinksTask::default());
    scheduler.add_task(task);

    let task2: Arc<dyn MaintenanceTask> = Arc::new(CleanupUnusedLinksTask::new(30));
    scheduler.add_task(task2);
}

#[sqlx::test]
async fn test_maintenance_task_default_should_run_implementation(pool: PgPool) {
    // Test the default should_run implementation
    struct TestTaskWithDefault;

    #[async_trait::async_trait]
    impl MaintenanceTask for TestTaskWithDefault {
        fn name(&self) -> &'static str {
            "test_task_default"
        }

        async fn execute(
            &self,
            _pool: &sqlx::Pool<sqlx::Postgres>,
            _usage_metrics: &dyn UsageMetrics,
            _cache: &dyn Cache,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        // Uses default should_run implementation
    }

    let task = TestTaskWithDefault;
    let metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool));

    // Default implementation checks load < 0.7
    let should_run = task.should_run(metrics.as_ref()).await;
    assert!(should_run.is_ok());
}

#[sqlx::test]
async fn test_cleanup_task_integrates_with_cache(pool: PgPool) {
    // Test that cleanup task calls cache.invalidate_all when links are deleted
    struct TestCache {
        invalidate_all_called: Arc<std::sync::Mutex<bool>>,
    }

    #[async_trait::async_trait]
    impl Cache for TestCache {
        async fn invalidate(&self, _key: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn invalidate_all(&self) -> anyhow::Result<()> {
            *self.invalidate_all_called.lock().unwrap() = true;
            Ok(())
        }
    }

    let invalidate_called = Arc::new(std::sync::Mutex::new(false));
    let cache: Arc<dyn Cache> = Arc::new(TestCache {
        invalidate_all_called: invalidate_called.clone(),
    });
    let metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));

    // Create an old link to delete
    let old_alias = "cache_test_link";
    sqlx::query(
        r#"
        INSERT INTO links (alias, url, created_at)
        VALUES ($1, $2, now() - interval '1 day')
        "#,
    )
    .bind(old_alias)
    .bind("https://cache-test.example.com")
    .execute(&pool)
    .await
    .unwrap();

    let task = CleanupUnusedLinksTask::new(0); // 0 days for testing

    // Execute cleanup
    assert!(
        task.execute(&pool, metrics.as_ref(), cache.as_ref())
            .await
            .is_ok()
    );

    // Verify cache.invalidate_all was called
    assert!(
        *invalidate_called.lock().unwrap(),
        "Cache invalidate_all should have been called"
    );

    // Cleanup
    sqlx::query("DELETE FROM links WHERE alias = $1")
        .bind(old_alias)
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_cleanup_task_does_not_call_cache_when_nothing_deleted(pool: PgPool) {
    // Test that cleanup task doesn't call cache when no links are deleted
    struct TestCache {
        invalidate_all_called: Arc<std::sync::Mutex<bool>>,
    }

    #[async_trait::async_trait]
    impl Cache for TestCache {
        async fn invalidate(&self, _key: &str) -> anyhow::Result<()> {
            Ok(())
        }

        async fn invalidate_all(&self) -> anyhow::Result<()> {
            *self.invalidate_all_called.lock().unwrap() = true;
            Ok(())
        }
    }

    let invalidate_called = Arc::new(std::sync::Mutex::new(false));
    let cache: Arc<dyn Cache> = Arc::new(TestCache {
        invalidate_all_called: invalidate_called.clone(),
    });
    let metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));

    // Create a recent link that won't be deleted
    let recent_alias = "cache_test_recent";
    sqlx::query(
        r#"
        INSERT INTO links (alias, url, created_at, last_accessed_at)
        VALUES ($1, $2, now(), now())
        "#,
    )
    .bind(recent_alias)
    .bind("https://cache-test-recent.example.com")
    .execute(&pool)
    .await
    .unwrap();

    let task = CleanupUnusedLinksTask::new(90); // 90 days - won't delete recent link

    // Execute cleanup
    assert!(
        task.execute(&pool, metrics.as_ref(), cache.as_ref())
            .await
            .is_ok()
    );

    // Verify cache.invalidate_all was NOT called (no links deleted)
    assert!(
        !*invalidate_called.lock().unwrap(),
        "Cache invalidate_all should NOT have been called when no links deleted"
    );

    // Cleanup
    sqlx::query("DELETE FROM links WHERE alias = $1")
        .bind(recent_alias)
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_multiple_cleanup_tasks_with_different_thresholds(pool: PgPool) {
    // Test that we can have multiple cleanup tasks with different day thresholds
    let cache: Arc<dyn Cache> = Arc::new(NoOpCache);
    let metrics: Arc<dyn UsageMetrics> = Arc::new(DefaultUsageMetrics::new(pool.clone()));

    // Create links of different ages
    let very_old = "very_old_test";
    let moderately_old = "moderately_old_test";
    let recent = "recent_test";

    sqlx::query(
        r#"
        INSERT INTO links (alias, url, created_at) VALUES
        ($1, 'https://very-old.example.com', now() - interval '100 days'),
        ($2, 'https://moderately-old.example.com', now() - interval '50 days'),
        ($3, 'https://recent.example.com', now() - interval '10 days')
        "#,
    )
    .bind(very_old)
    .bind(moderately_old)
    .bind(recent)
    .execute(&pool)
    .await
    .unwrap();

    // Run cleanup with 30-day threshold (should delete very_old and moderately_old)
    let task_30 = CleanupUnusedLinksTask::new(30);
    assert!(
        task_30
            .execute(&pool, metrics.as_ref(), cache.as_ref())
            .await
            .is_ok()
    );

    // Verify very_old and moderately_old were deleted
    let very_old_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1")
        .bind(very_old)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(very_old_count.0, 0);

    let moderately_old_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1")
            .bind(moderately_old)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(moderately_old_count.0, 0);

    // Verify recent still exists
    let recent_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1")
        .bind(recent)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(recent_count.0, 1);

    // Cleanup
    sqlx::query("DELETE FROM links WHERE alias = $1 OR alias = $2 OR alias = $3")
        .bind(very_old)
        .bind(moderately_old)
        .bind(recent)
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_app_get_url_records_access_metrics(pool: PgPool) {
    // Integration test: verify that AppState.get_url records access metrics
    use url_shorten::app;

    let app_state = app::build_app_state(pool.clone()).await.unwrap();

    // Create a link
    let url = "https://metrics-test.example.com";
    let alias = app_state.shorten_url(url).await.unwrap();

    // Verify last_accessed_at is initially NULL
    let initial_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM links WHERE alias = $1 AND last_accessed_at IS NULL")
            .bind(&alias)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        initial_count.0, 1,
        "Link should exist with NULL last_accessed_at initially"
    );

    // Call get_url which should record access
    let retrieved_url = app_state.get_url(&alias).await.unwrap();
    assert_eq!(retrieved_url, url);

    // Verify last_accessed_at was updated
    let updated_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM links WHERE alias = $1 AND last_accessed_at IS NOT NULL",
    )
    .bind(&alias)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        updated_count.0, 1,
        "last_accessed_at should have been set after get_url"
    );

    // Cleanup
    sqlx::query("DELETE FROM links WHERE alias = $1")
        .bind(&alias)
        .execute(&pool)
        .await
        .unwrap();
}
