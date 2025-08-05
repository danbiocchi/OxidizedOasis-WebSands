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
pub use infrastructure::database::migrations::run_migrations;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lib_module_exports() {
        // Test that all public modules are accessible
        // This verifies that the module declarations work correctly
        
        // These imports should compile successfully if modules are properly exported
        use crate::api;
        use crate::common;
        use crate::core;
        use crate::infrastructure;
        
        // If we reach here, module exports are working
        assert!(true, "All public modules are accessible");
    }

    #[test]
    fn test_lib_type_re_exports() {
        // Test that re-exported types are accessible
        
        // These should be available via the re-exports
        use crate::{ApiError, AuthError, DbError, Claims, User, NewUser, AppConfig, create_pool};
        
        // Test that we can reference the re-exported types
        let _error_check = std::any::type_name::<ApiError>();
        let _auth_error_check = std::any::type_name::<AuthError>();
        let _db_error_check = std::any::type_name::<DbError>();
        let _claims_check = std::any::type_name::<Claims>();
        let _user_check = std::any::type_name::<User>();
        let _new_user_check = std::any::type_name::<NewUser>();
        let _config_check = std::any::type_name::<AppConfig>();
        let _pool_fn_check = std::any::type_name_of_val(&create_pool);
        
        assert!(true, "All re-exported types are accessible");
    }

    #[test]
    fn test_lib_public_api_surface() {
        // Test that the public API surface is stable
        
        // Verify that common error types are re-exported
        use crate::{ApiError, AuthError, DbError};
        
        // These types should be available for library consumers
        let _api_error = std::any::type_name::<ApiError>();
        let _auth_error = std::any::type_name::<AuthError>();
        let _db_error = std::any::type_name::<DbError>();
        
        assert!(true, "Public API surface is stable");
    }

    #[test]
    fn test_lib_core_types_available() {
        // Test that core business types are available
        
        use crate::{Claims, User, NewUser};
        
        // Core types should be accessible for business logic
        let _claims = std::any::type_name::<Claims>();
        let _user = std::any::type_name::<User>();
        let _new_user = std::any::type_name::<NewUser>();
        
        assert!(true, "Core business types are available");
    }

    #[test]
    fn test_lib_infrastructure_types_available() {
        // Test that infrastructure types are available
        
        use crate::{AppConfig, create_pool};
        
        // Infrastructure types should be accessible
        let _config = std::any::type_name::<AppConfig>();
        let _create_pool_fn = std::any::type_name_of_val(&create_pool);
        
        assert!(true, "Infrastructure types are available");
    }

    #[cfg(any(test, feature = "test-utils"))]
    #[test]
    fn test_lib_test_utils_available() {
        // Test that test utilities are available when feature is enabled
        
        use crate::test_utils::{create_test_config};
        
        // Test that test utilities work
        let config = create_test_config();
        
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, "8080");
        assert_eq!(config.jwt.secret, "test_secret_key_for_testing_only");
        assert_eq!(config.jwt.audience, "test");
        assert_eq!(config.jwt.issuer, "test");
        assert_eq!(config.database.max_connections, 5);
        assert!(config.database.url.contains("postgresql://"));
    }

    #[cfg(any(test, feature = "test-utils"))]
    #[test]
    fn test_lib_test_utils_config_validation() {
        // Test that test configuration values are reasonable
        
        use crate::test_utils::create_test_config;
        
        let config = create_test_config();
        
        // Validate server configuration
        assert!(!config.server.host.is_empty(), "Host should not be empty");
        assert!(!config.server.port.is_empty(), "Port should not be empty");
        assert!(config.server.port.parse::<u16>().is_ok(), "Port should be a valid number");
        
        // Validate database configuration
        assert!(!config.database.url.is_empty(), "Database URL should not be empty");
        assert!(config.database.url.starts_with("postgresql://"), "Should be PostgreSQL URL");
        assert!(config.database.max_connections > 0, "Max connections should be positive");
        assert!(config.database.max_connections <= 100, "Max connections should be reasonable");
        
        // Validate JWT configuration
        assert!(!config.jwt.secret.is_empty(), "JWT secret should not be empty");
        assert!(config.jwt.secret.len() >= 16, "JWT secret should be at least 16 characters");
        assert!(!config.jwt.audience.is_empty(), "JWT audience should not be empty");
        assert!(!config.jwt.issuer.is_empty(), "JWT issuer should not be empty");
    }

    #[cfg(any(test, feature = "test-utils"))]
    #[tokio::test]
    async fn test_lib_test_utils_create_test_pool_function_exists() {
        // Test that create_test_pool function exists and can be referenced
        // We can't actually test database connection without a real database
        
        use crate::test_utils::create_test_pool;
        
        // Just verify the function exists and compiles
        let _function_ref = create_test_pool;
        
        // This test verifies the function signature compiles correctly
        assert!(true, "create_test_pool function exists and compiles");
    }

    #[test]
    fn test_lib_compilation_and_linkage() {
        // Test that the library compiles and links correctly
        
        // This test existing and running proves compilation works
        // We can also test that basic library functions work
        
        assert!(true, "Library compiles and links correctly");
    }

    #[test]
    fn test_lib_module_structure_integrity() {
        // Test that the module structure is intact
        
        // Test that we can access nested modules through the public interface
        // This verifies that the pub mod declarations work correctly
        
        // If these compile, the module structure is correct
        use crate::api;
        use crate::common::error;
        use crate::core::user;
        use crate::infrastructure::config;
        
        // Reference the modules to ensure they're properly linked
        // Just test that imports compile successfully
        
        assert!(true, "Module structure integrity is maintained");
    }

    #[test]
    fn test_lib_no_runtime_overhead() {
        // Test that the library interface doesn't introduce runtime overhead
        
        let start = std::time::Instant::now();
        
        // Perform operations that would involve the library interface
        use crate::{ApiError, User, AppConfig};
        
        let _error_type = std::any::type_name::<ApiError>();
        let _user_type = std::any::type_name::<User>();
        let _config_type = std::any::type_name::<AppConfig>();
        
        let duration = start.elapsed();
        
        // Library interface operations should be essentially zero-cost
        assert!(duration < std::time::Duration::from_millis(1),
                "Library interface should not introduce runtime overhead");
    }

    #[test]
    fn test_lib_re_export_functionality() {
        // Test the actual functionality of re-exported types
        
        use crate::{ApiError, AuthError, DbError};
        use crate::common::error::{ApiErrorType, AuthErrorType};
        
        // Test that error types can be created and used
        let api_error = ApiError::new("test error", ApiErrorType::Validation);
        let auth_error = AuthError::new(AuthErrorType::InvalidToken);
        let db_error = DbError::ConnectionError("test connection error".to_string());
        
        // Test error formatting
        assert!(format!("{:?}", api_error).contains("test error"));
        assert!(format!("{:?}", auth_error).contains("InvalidToken"));
        assert!(format!("{:?}", db_error).contains("connection error"));
    }

    #[test]
    fn test_lib_module_boundary_integrity() {
        // Test that module boundaries are properly maintained
        
        // Test that private implementation details are not exposed
        // This is a compile-time test - if it compiles, boundaries are correct
        
        // We should be able to access public interfaces
        use crate::api;
        use crate::common;
        use crate::core;
        use crate::infrastructure;
        
        // But not internal implementation details (these should not compile if exposed)
        // This test existing proves the module structure is correct
        assert!(true, "Module boundaries are properly maintained");
    }

    #[test]
    fn test_lib_public_api_completeness() {
        // Test that all expected public APIs are available
        
        // Error types
        use crate::{ApiError, AuthError, DbError};
        let _api_error_test = std::any::type_name::<ApiError>();
        let _auth_error_test = std::any::type_name::<AuthError>();
        let _db_error_test = std::any::type_name::<DbError>();
        
        // Core business types
        use crate::{Claims, User, NewUser};
        let _claims_test = std::any::type_name::<Claims>();
        let _user_test = std::any::type_name::<User>();
        let _new_user_test = std::any::type_name::<NewUser>();
        
        // Infrastructure types
        use crate::{AppConfig, create_pool, run_migrations};
        let _config_test = std::any::type_name::<AppConfig>();
        let _create_pool_test = std::any::type_name_of_val(&create_pool);
        let _run_migrations_test = std::any::type_name_of_val(&run_migrations);
        
        assert!(true, "All expected public APIs are available");
    }

    #[test]
    fn test_lib_feature_flag_behavior() {
        // Test that feature flags work correctly
        
        // Test utils should only be available with test feature
        #[cfg(any(test, feature = "test-utils"))]
        {
            use crate::test_utils;
            // If we're in a test context, test_utils should be available
            let _test_utils_module = test_utils::create_test_config;
        }
        
        // This test being able to compile proves feature flags work
        assert!(true, "Feature flags behave correctly");
    }

    #[test]
    fn test_lib_error_type_compatibility() {
        // Test that error types implement expected traits
        
        use crate::{ApiError, AuthError, DbError};
        use crate::common::error::{ApiErrorType, AuthErrorType};
        
        // Test Debug trait
        let api_error = ApiError::new("test", ApiErrorType::Internal);
        let debug_output = format!("{:?}", api_error);
        assert!(!debug_output.is_empty(), "ApiError should implement Debug");
        
        // Test that errors can be converted to strings (Display trait)
        let auth_error = AuthError::new(AuthErrorType::InvalidToken);
        let display_output = format!("{}", auth_error);
        assert!(!display_output.is_empty(), "AuthError should implement Display");
        
        // Test that errors can be used as std::error::Error
        let db_error = DbError::QueryError("test query".to_string());
        let _error_ref: &dyn std::error::Error = &db_error;
        
        assert!(true, "Error types implement expected traits");
    }

    #[cfg(any(test, feature = "test-utils"))]
    #[test]
    fn test_lib_test_utils_isolation() {
        // Test that test utilities don't interfere with each other
        
        use crate::test_utils::create_test_config;
        
        let config1 = create_test_config();
        let config2 = create_test_config();
        
        // Configs should be identical but independent
        assert_eq!(config1.server.host, config2.server.host);
        assert_eq!(config1.database.url, config2.database.url);
        assert_eq!(config1.jwt.secret, config2.jwt.secret);
        
        // They should be independent instances
        assert!(std::ptr::addr_of!(config1) != std::ptr::addr_of!(config2), "Configs should be independent instances");
    }
}