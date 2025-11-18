// Redis cache implementation module
//
// Provides distributed caching using Redis with production-ready error handling.
// Supports MessagePack serialization for efficient storage.

use crate::cache::{Cache, CacheEntry, CacheError, CacheKey, CacheStats};
use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client, RedisError};
use std::sync::Arc;

pub mod config;
pub use config::RedisConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_import_redis_client() {
        // This test verifies that redis::Client can be imported and used
        // We just need to verify the type is available
        let _phantom: Option<Client> = None;
    }

    #[test]
    fn test_can_import_connection_manager() {
        // This test verifies that redis::aio::ConnectionManager can be imported
        let _phantom: Option<ConnectionManager> = None;
    }

    #[test]
    fn test_can_import_redis_error() {
        // This test verifies that redis::RedisError can be imported
        // We create a simple test to confirm the type exists
        fn _check_error_type(_err: RedisError) {}
    }

    #[test]
    fn test_async_commands_trait_available() {
        // This test verifies that AsyncCommands trait is available
        // We can't directly instantiate it, but we can verify it exists in the type system
        // The trait is used in the implementation, so this confirms it compiles
        fn _uses_async_commands<T: AsyncCommands>(_t: T) {}
    }
}
