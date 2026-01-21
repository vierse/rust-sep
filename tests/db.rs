use std::time::{SystemTime, UNIX_EPOCH};

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

#[tokio::test]
async fn test_create_user() {
    let config = config::load().expect("Could not load config");
    let pool = app::connect_to_db(config.database_url.as_str())
        .await
        .expect("Could not connect to DB");
    let db = Database::new(pool);

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let username = format!("test_user_{nonce}");

    let user_id = db
        .create_user(&username, "p@ssw0rd")
        .await
        .expect("failed to create user");

    assert!(user_id > 0, "user id should be positive");
}

#[tokio::test]
async fn test_verify_user_password() {
    let config = config::load().expect("Could not load config");
    let pool = app::connect_to_db(config.database_url.as_str())
        .await
        .expect("Could not connect to DB");
    let db = Database::new(pool);

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let username = format!("test_user_verify_{nonce}");
    let password = "s3cr3t";

    let user_id = db
        .create_user(&username, password)
        .await
        .expect("failed to create user");

    let ok = db
        .verify_user_password(&username, password)
        .await
        .expect("failed to verify password");
    assert_eq!(ok, Some(user_id), "should return user id on success");

    let wrong = db
        .verify_user_password(&username, "wrong_password")
        .await
        .expect("failed to verify wrong password");
    assert!(wrong.is_none(), "should return None for wrong password");
}
