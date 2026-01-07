use url_shorten::{app, config, db::Database};

#[tokio::test]
async fn test_remove() {
    let config = config::load().expect("Could not load config");
    let pool = app::connect_to_db(config.database_url.as_str())
        .await
        .expect("Could not connect to DB");
    let db = Database::new(pool.clone());
    let state = app::build_app_state(pool.clone()).await.unwrap();

    // removing nonexistent alias returns false
    let removed = db.remove("nonexistent_alias_to_remove").await.unwrap();
    assert!(!removed, "should return false for nonexistent alias");

    // removing existent alias returns true
    let alias = state.shorten_url("https://a_url.com").await.unwrap();
    let removed = db.remove(&alias).await.unwrap();
    assert!(removed, "should return true for existent alias");
}
