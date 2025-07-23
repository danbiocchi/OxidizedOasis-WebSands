//! Oasis Library
//! 
//! This library provides the core functionality for the Oasis application,
//! including authentication, user management, email services, and API handlers.

// Public modules for library consumers and tests
pub mod api;
pub mod common;
pub mod core;
pub mod infrastructure;

// Re-export commonly used types for convenience
pub use common::error::{ApiError, AuthError, DbError};
pub use core::auth::jwt::Claims;
pub use core::user::model::{User, NewUser};
pub use infrastructure::config::app_config::AppConfig;
pub use infrastructure::database::connection::create_pool;

// Test utilities - available for tests and integration tests
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use crate::infrastructure::config::app_config::{AppConfig, DatabaseConfig, JwtConfig, ServerConfig};
    use std::sync::Arc;
    use sqlx::PgPool;
    
    /// Create a test configuration for testing
    pub fn create_test_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: "8080".to_string(),
            },
            database: DatabaseConfig {
                url: "postgresql://test:test@localhost:5432/test".to_string(),
                max_connections: 5,
            },
            jwt: JwtConfig {
                secret: "test_secret_key_for_testing_only".to_string(),
                audience: "test".to_string(),
                issuer: "test".to_string(),
            },
        }
    }
    
    /// Create a test database pool
    pub async fn create_test_pool() -> Result<Arc<PgPool>, sqlx::Error> {
        let config = create_test_config();
        crate::infrastructure::database::connection::create_pool(&config).await
            .map(|pool| Arc::new(pool))
            .map_err(|e| sqlx::Error::Configuration(Box::new(e)))
    }
}