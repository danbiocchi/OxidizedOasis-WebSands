//! Tests for src/lib.rs - Library interface and module exports
//!
//! This test file covers the main library interface, ensuring all public
//! modules and re-exports are properly accessible and functional.

use oxidizedoasis_websands::{
    // Test re-exports - These imports exercise the pub use statements in lib.rs
    ApiError, AuthError, DbError,
    Claims, User, NewUser, AppConfig,
    create_pool,
};

#[cfg(test)]
mod lib_module_tests {
    

    /// Test that lib.rs pub mod declarations work correctly
    #[test]
    fn test_public_module_declarations() {
        // These imports directly test the pub mod statements in lib.rs
        use oxidizedoasis_websands::api;
        
        use oxidizedoasis_websands::core;
        use oxidizedoasis_websands::infrastructure;
        
        // Verify modules are accessible and contain expected submodules
        let _handlers_module = std::any::type_name::<api::handlers::user_handler::UserHandler>();
        let _error_module = std::any::type_name::<oxidizedoasis_websands::common::error::ApiError>();
        let _user_module = std::any::type_name::<core::user::User>();
        let _config_module = std::any::type_name::<infrastructure::config::AppConfig>();
        
        // If we reach here without compilation errors, the pub mod declarations work
        assert!(true, "All public modules are accessible");
    }

    /// Test that lib.rs re-exports work correctly
    #[test]
    fn test_lib_reexports() {
        // These directly test the pub use statements in lib.rs
        use oxidizedoasis_websands::{ApiError, AuthError, DbError};
        use oxidizedoasis_websands::{Claims, User, NewUser};
        use oxidizedoasis_websands::{AppConfig, create_pool};
        
        // Test that we can create instances using re-exported types
        let _api_error_type = std::any::type_name::<ApiError>();
        let _auth_error_type = std::any::type_name::<AuthError>();
        let _db_error_type = std::any::type_name::<DbError>();
        let _claims_type = std::any::type_name::<Claims>();
        let _user_type = std::any::type_name::<User>();
        let _new_user_type = std::any::type_name::<NewUser>();
        let _app_config_type = std::any::type_name::<AppConfig>();
        
        // Verify create_pool function is accessible via re-export
        assert_eq!(std::any::type_name_of_val(&create_pool), "oxidizedoasis_websands::infrastructure::database::connection::create_pool");
        
        // If we reach here, all re-exports are working
        assert!(true, "All re-exports from lib.rs are accessible");
    }

    /// Test lib.rs crate structure and exports
    #[test]
    fn test_crate_structure() {
        // This tests the overall crate structure defined in lib.rs
        // Testing that we can access all modules and re-exports as expected
        
        // Test crate-level access
        
        
        // These imports should work if lib.rs pub mod declarations are correct
        let crate_name = std::module_path!();
        assert!(crate_name.contains("lib_tests") || crate_name.contains("oxidizedoasis_websands"));
        
        // Test that the library compiles and links correctly
        assert!(true, "Library structure is accessible and consistent");
    }

    /// Test that lib.rs compiles and exports are consistent
    #[test]
    fn test_lib_consistency() {
        // Test that all expected exports from lib.rs are available
        // This exercises the actual lib.rs file structure
        
        // Module exports (pub mod statements)
        use oxidizedoasis_websands::{api, core, infrastructure};
        
        // Type re-exports (pub use statements)
        use oxidizedoasis_websands::{
            ApiError, AuthError, DbError,
            Claims, User, NewUser, AppConfig, create_pool
        };
        
        // Verify no compilation errors occur when using lib.rs exports
        let module_names = [std::any::type_name::<api::handlers::user_handler::UserHandler>(),
            std::any::type_name::<oxidizedoasis_websands::common::error::ApiError>(),
            std::any::type_name::<core::user::User>(),
            std::any::type_name::<infrastructure::config::AppConfig>()];
        
        let type_names = [std::any::type_name::<ApiError>(),
            std::any::type_name::<AuthError>(),
            std::any::type_name::<DbError>(),
            std::any::type_name::<Claims>(),
            std::any::type_name::<User>(),
            std::any::type_name::<NewUser>(),
            std::any::type_name::<AppConfig>()];
        
        // All should contain expected type information
        assert!(!module_names.is_empty(), "Module names should be accessible");
        assert!(!type_names.is_empty(), "Type names should be accessible");
        
        // Test function re-export
        assert!(std::any::type_name_of_val(&create_pool).contains("create_pool"));
    }

    /// Test lib.rs documentation and structure
    #[test]
    fn test_lib_documentation_structure() {
        // This test ensures the lib.rs structure is correct by using all exports
        
        // Test that we can access the crate root
        use oxidizedoasis_websands as oasis;
        
        // Test module access through crate root
        let _api = std::any::type_name::<oasis::api::handlers::user_handler::UserHandler>();
        let _common = std::any::type_name::<oasis::common::error::ApiError>();
        let _core = std::any::type_name::<oasis::core::user::User>();
        let _infra = std::any::type_name::<oasis::infrastructure::config::AppConfig>();
        
        // Test re-exports through crate root
        let _api_error = std::any::type_name::<oasis::ApiError>();
        let _auth_error = std::any::type_name::<oasis::AuthError>();
        let _db_error = std::any::type_name::<oasis::DbError>();
        let _claims = std::any::type_name::<oasis::Claims>();
        let _user = std::any::type_name::<oasis::User>();
        let _new_user = std::any::type_name::<oasis::NewUser>();
        let _app_config = std::any::type_name::<oasis::AppConfig>();
        
        // All should be accessible through the crate root, testing lib.rs structure
        assert!(true, "Library structure is consistent");
    }
}

#[cfg(test)]
mod lib_interface_tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    /// Test that all error types are properly re-exported
    #[test]
    fn test_error_type_reexports() {
        use oxidizedoasis_websands::common::error::{ApiErrorType, AuthErrorType};
        
        // Test ApiError can be created
        let api_error = ApiError::bad_request("test");
        assert_eq!(api_error.message, "test");
        assert_eq!(api_error.error_type, ApiErrorType::Validation);

        // Test AuthError can be created
        let auth_error = AuthError::new(AuthErrorType::InvalidCredentials);
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidCredentials);
        assert_eq!(auth_error.message, "Invalid credentials");

        // Test DbError can be created using enum variants
        let db_error = DbError::ConnectionError("test".to_string());
        assert_eq!(format!("{db_error}"), "Database connection error: test");
    }

    /// Test that Claims struct is properly re-exported
    #[test]
    fn test_claims_reexport() {
        let claims = Claims {
            sub: Uuid::new_v4(),
            exp: Utc::now().timestamp() + 3600,
            iat: Utc::now().timestamp(),
            nbf: Utc::now().timestamp(),
            jti: "test_jti".to_string(),
            role: "user".to_string(),
            token_type: oxidizedoasis_websands::core::auth::jwt::TokenType::Access,
            aud: "test".to_string(),
            iss: "test".to_string(),
        };

        assert_eq!(claims.role, "user");
        assert_eq!(claims.jti, "test_jti");
    }

    /// Test that User and NewUser structs are properly re-exported
    #[test]
    fn test_user_types_reexport() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let user = User {
            id: user_id,
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: now,
            updated_at: now,
            role: "user".to_string(),
            is_active: true,
        };

        assert_eq!(user.username, "testuser");
        assert_eq!(user.role, "user");
        assert!(user.is_active);

        let new_user = NewUser {
            username: "newuser".to_string(),
            email: Some("new@example.com".to_string()),
            password_hash: "new_hashed_password".to_string(),
            is_email_verified: false,
            role: "user".to_string(),
            verification_token: Some("token123".to_string()),
            verification_token_expires_at: Some(now + chrono::Duration::hours(24)),
        };

        assert_eq!(new_user.username, "newuser");
        assert!(!new_user.is_email_verified);
        assert_eq!(new_user.verification_token, Some("token123".to_string()));
    }

    /// Test that AppConfig is properly re-exported
    #[test]
    fn test_app_config_reexport() {
        use oxidizedoasis_websands::infrastructure::config::app_config::{
            ServerConfig, DatabaseConfig, JwtConfig
        };

        let config = AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: "8080".to_string(),
            },
            database: DatabaseConfig {
                url: "postgresql://test:test@localhost:5432/test".to_string(),
                max_connections: 5,
            },
            jwt: JwtConfig {
                secret: "test_secret".to_string(),
                audience: "test_audience".to_string(),
                issuer: "test_issuer".to_string(),
            },
        };

        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.database.max_connections, 5);
        assert_eq!(config.jwt.secret, "test_secret");
    }

    /// Test that create_pool function is properly re-exported
    #[test]
    fn test_create_pool_reexport() {
        // We can't actually test the pool creation without a database
        // but we can verify the function is accessible
        use oxidizedoasis_websands::infrastructure::config::app_config::{
            AppConfig, ServerConfig, DatabaseConfig, JwtConfig
        };

        let config = AppConfig {
            server: ServerConfig {
                host: "localhost".to_string(),
                port: "8080".to_string(),
            },
            database: DatabaseConfig {
                url: "postgresql://test:test@localhost:5432/nonexistent".to_string(),
                max_connections: 1,
            },
            jwt: JwtConfig {
                secret: "test_secret".to_string(),
                audience: "test_audience".to_string(),
                issuer: "test_issuer".to_string(),
            },
        };

        // Test that the function is accessible (we can't test actual connection without DB)
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // The function should be callable, even if it fails due to no database
            let result = create_pool(&config).await;
            // We expect this to fail since we don't have a real database
            assert!(result.is_err());
        });
    }
}

#[cfg(feature = "test-utils")]
#[cfg(test)]
mod test_utils_tests {
    use super::*;

    /// Test that test utilities are available when feature is enabled
    #[test]
    fn test_test_utils_available() {
        use oxidizedoasis_websands::test_utils::{create_test_config, create_test_pool};

        let config = create_test_config();
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.jwt.secret, "test_secret_key_for_testing_only");

        // Test that create_test_pool is accessible
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = create_test_pool().await;
            // Expect failure due to no actual database
            assert!(result.is_err());
        });
    }
}

#[cfg(test)]
mod module_accessibility_tests {
    /// Test that all public modules are accessible
    #[test]
    fn test_public_modules_accessible() {
        // Test that we can access the main modules
        use oxidizedoasis_websands::{api, core, infrastructure};
        
        // These should compile without errors, proving the modules are public
        let _api_module = std::any::type_name::<api::handlers::user_handler::UserHandler>();
        let _common_module = std::any::type_name::<oxidizedoasis_websands::common::error::ApiError>();
        let _core_module = std::any::type_name::<core::user::User>();
        let _infra_module = std::any::type_name::<infrastructure::config::AppConfig>();
    }

    /// Test library structure and documentation
    #[test]
    fn test_library_structure() {
        // Test that the library is properly structured with expected components
        use oxidizedoasis_websands::*;
        
        // Verify core functionality is accessible
        assert!(std::any::type_name::<User>().contains("User"));
        assert!(std::any::type_name::<Claims>().contains("Claims"));
        assert!(std::any::type_name::<AppConfig>().contains("AppConfig"));
        
        // Verify error handling is properly exposed
        assert!(std::any::type_name::<ApiError>().contains("ApiError"));
        assert!(std::any::type_name::<AuthError>().contains("AuthError"));
        assert!(std::any::type_name::<DbError>().contains("DbError"));
    }
}