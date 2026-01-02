use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use url_shorten::core::{App, BaseApp};
use url_shorten::db::Database;

mod common;

#[derive(Clone)]
struct TestDB {

    data: Arc<Mutex<std::collections::HashMap<String, String>>>,
    should_fail_insert: Arc<Mutex<bool>>,
    insert_error_count: Arc<Mutex<u32>>,
}

impl TestDB {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(std::collections::HashMap::new())),
            should_fail_insert: Arc::new(Mutex::new(false)),
            insert_error_count: Arc::new(Mutex::new(0)),
        }
    }

    fn set_should_fail_insert(&self, should_fail: bool) {
        *self.should_fail_insert.lock().unwrap() = should_fail;
    }

    fn set_insert_error_count(&self, count: u32) {
        *self.insert_error_count.lock().unwrap() = count;
    }
}

#[async_trait]
impl Database for TestDB {
    async fn insert(&self, alias: &str, url: &str) -> Result<()> {
        let mut error_count = self.insert_error_count.lock().unwrap();
        if *error_count > 0 {
            *error_count -= 1;
            return Err(anyhow!("Simulated insert error"));
        }

        if *self.should_fail_insert.lock().unwrap() {
            return Err(anyhow!("Simulated insert failure"));
        }

        let mut data = self.data.lock().unwrap();
        data.insert(alias.to_string(), url.to_string());
        Ok(())
    }

    async fn get(&self, alias: &str) -> Result<String> {
        let data = self.data.lock().unwrap();
        data.get(alias)
            .cloned()
            .ok_or_else(|| anyhow!("Alias not found"))
    }
}

#[tokio::test]
async fn test_shorten_url_success() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db);

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_ok());
    let alias = result.unwrap();
    assert_eq!(alias.len(), 6); 

    
    let retrieved = app.get_url(&alias).await;
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap(), url);
}

#[tokio::test]
async fn test_get_url_success() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db.clone());

    // Insert a URL directly through the database
    db.insert("test123", "https://www.test.com").await.unwrap();

    let result = app.get_url("test123").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "https://www.test.com");
}

#[tokio::test]
async fn test_get_url_not_found() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db);

    let result = app.get_url("nonexistent").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Alias not found"));
}

#[tokio::test]
async fn test_shorten_url_retry_on_collision() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db.clone());

    
    for i in 0..100 {
        let url = format!("https://www.example{}.com", i);
        let result = app.shorten_url(&url).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_shorten_url_handles_insert_error() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db.clone());

    db.set_insert_error_count(1);

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;


    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shorten_url_handles_concurrent_insert() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db.clone());


    db.set_insert_error_count(1);

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_shorten_url_multiple_urls() {
    let db = Arc::new(TestDB::new());
    let app = App::new(db);

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
            assert_ne!(aliases[i], aliases[j]);
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
    let db = Arc::new(TestDB::new());
    let app = App::new(db);


    for i in 0..10 {
        let url = format!("https://www.example{}.com", i);
        let alias = app.shorten_url(&url).await.unwrap();
        assert_eq!(alias.len(), 6, "Alias should be exactly 6 characters");
    }
}

#[derive(Clone)]
struct AlwaysCollidingDB;

#[async_trait]
impl Database for AlwaysCollidingDB {
    async fn insert(&self, _alias: &str, _url: &str) -> Result<()> {
        Ok(())
    }

    async fn get(&self, _alias: &str) -> Result<String> {
    
        Ok("https://existing.com".to_string())
    }
}

#[tokio::test]
async fn test_shorten_url_max_retries_exceeded() {
    let db = Arc::new(AlwaysCollidingDB);
    let app = App::new(db);

    let url = "https://www.example.com";
    let result = app.shorten_url(url).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to generate unique alias after"));
}

