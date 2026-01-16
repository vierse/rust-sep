use anyhow::Result;
use async_trait::async_trait;

/// Trait for cache operations
///
/// This is designed to integrate with Cache #14.
/// Provides cache invalidation capabilities for maintenance tasks.
#[async_trait]
pub trait Cache: Send + Sync {
    /// Invalidate a specific cache entry by key
    async fn invalidate(&self, key: &str) -> Result<()>;

    /// Invalidate all cache entries
    async fn invalidate_all(&self) -> Result<()>;
}

/// Default no-op cache implementation
///
/// Used when Cache #14 is not yet implemented.
pub struct NoOpCache;

#[async_trait]
impl Cache for NoOpCache {
    async fn invalidate(&self, _key: &str) -> Result<()> {
        // No-op: Cache #14 not implemented yet
        Ok(())
    }

    async fn invalidate_all(&self) -> Result<()> {
        // No-op: Cache #14 not implemented yet
        Ok(())
    }
}
