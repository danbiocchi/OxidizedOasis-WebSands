//! Common test utilities and infrastructure

use oxidizedoasis_websands::test_utils::create_test_config;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::infrastructure::database::connection::create_pool;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{Utc, Duration};
use oxidizedoasis_websands::core::user::{User, NewUser};
use oxidizedoasis_websands::core::auth::jwt::{Claims, TokenType, create_token_pair};

pub mod database;

/// Test configuration constants
pub const TEST_JWT_SECRET: &str = "test_secret_key_for_comprehensive_testing_12345";
pub const TEST_AUDIENCE: &str = "test_audience";
pub const TEST_ISSUER: &str = "test_issuer";

/// Create test configuration for integration tests
pub fn create_test_app_config() -> AppConfig {
    create_test_config()
}

/// Create test database pool for integration tests
pub async fn create_test_db_pool() -> Result<Arc<PgPool>, Box<dyn std::error::Error>> {
    let config = create_test_config();
    let pool = create_pool(&config).await?;
    Ok(Arc::new(pool))
}

/// Create a test user for various test scenarios
pub fn create_test_user(id: Uuid, username: &str, email: &str, is_verified: bool, role: &str) -> User {
    User {
        id,
        username: username.to_string(),
        email: Some(email.to_string()),
        password_hash: bcrypt::hash("TestPassword123!", bcrypt::DEFAULT_COST).unwrap(),
        role: role.to_string(),
        is_active: true,
        is_email_verified: is_verified,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        verification_token: None,
        verification_token_expires_at: None,
    }
}

/// Create a new user data structure for testing user creation
pub fn create_test_new_user(username: &str, email: &str, is_verified: bool) -> NewUser {
    NewUser {
        username: username.to_string(),
        email: Some(email.to_string()),
        password_hash: bcrypt::hash("TestPassword123!", bcrypt::DEFAULT_COST).unwrap(),
        role: "user".to_string(),
        is_email_verified: is_verified,
        verification_token: if is_verified { None } else { Some(Uuid::new_v4().to_string()) },
        verification_token_expires_at: if is_verified { None } else { Some(Utc::now() + Duration::days(1)) },
    }
}

/// Generate test JWT claims for various scenarios
pub fn create_test_claims(user_id: Uuid, role: &str, expires_in_secs: i64) -> Claims {
    let now = Utc::now();
    Claims {
        sub: user_id,
        exp: (now + Duration::seconds(expires_in_secs)).timestamp(),
        iat: now.timestamp(),
        nbf: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
        role: role.to_string(),
        token_type: TokenType::Access,
        aud: TEST_AUDIENCE.to_string(),
        iss: TEST_ISSUER.to_string(),
    }
}

/// Generate a valid test JWT token
pub fn generate_test_token(user_id: Uuid, role: &str, expires_in_secs: i64) -> Result<String, Box<dyn std::error::Error>> {
    let claims = create_test_claims(user_id, role, expires_in_secs);
    let header = jsonwebtoken::Header::default();
    let key = jsonwebtoken::EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes());
    
    jsonwebtoken::encode(&header, &claims, &key)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Generate test token pair for authentication tests
pub fn generate_test_token_pair(user_id: Uuid, role: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let token_pair = create_token_pair(user_id, role.to_string(), TEST_JWT_SECRET)?;
    Ok((token_pair.access_token, token_pair.refresh_token))
}

/// Test data constants
pub mod test_data {
    use uuid::Uuid;
    
    pub const TEST_USER_USERNAME: &str = "testuser";
    pub const TEST_USER_EMAIL: &str = "testuser@example.com";
    pub const TEST_USER_PASSWORD: &str = "TestPassword123!";
    
    pub const TEST_ADMIN_USERNAME: &str = "testadmin";
    pub const TEST_ADMIN_EMAIL: &str = "testadmin@example.com";
    
    pub const INVALID_EMAIL: &str = "invalid-email";
    pub const WEAK_PASSWORD: &str = "weak";
    
    pub fn test_user_id() -> Uuid {
        Uuid::new_v4()
    }
    
    pub fn test_admin_id() -> Uuid {
        Uuid::new_v4()
    }
}

/// HTTP test helpers
pub mod http {
    use actix_web::{test, App, web};
    use serde_json::Value;
    
    /// Extract JSON response body from test response
    pub async fn extract_json_response(response: actix_web::dev::ServiceResponse) -> Value {
        test::read_body_json(response).await
    }
    
    /// Create test request with JSON body
    pub fn create_json_request(method: &str, uri: &str, body: Value) -> test::TestRequest {
        match method {
            "POST" => test::TestRequest::post().uri(uri).set_json(&body),
            "PUT" => test::TestRequest::put().uri(uri).set_json(&body),
            "PATCH" => test::TestRequest::patch().uri(uri).set_json(&body),
            "DELETE" => test::TestRequest::delete().uri(uri).set_json(&body),
            _ => test::TestRequest::get().uri(uri),
        }
    }
    
    /// Create test request with Bearer token
    pub fn create_auth_request(method: &str, uri: &str, token: &str) -> test::TestRequest {
        let auth_header = format!("Bearer {}", token);
        match method {
            "POST" => test::TestRequest::post().uri(uri).insert_header(("Authorization", auth_header)),
            "PUT" => test::TestRequest::put().uri(uri).insert_header(("Authorization", auth_header)),
            "PATCH" => test::TestRequest::patch().uri(uri).insert_header(("Authorization", auth_header)),
            "DELETE" => test::TestRequest::delete().uri(uri).insert_header(("Authorization", auth_header)),
            _ => test::TestRequest::get().uri(uri).insert_header(("Authorization", auth_header)),
        }
    }
}

/// Environment variable helpers for tests
pub mod env {
    use std::sync::Mutex;
    use std::collections::HashMap;
    
    static ENV_MUTEX: Mutex<()> = Mutex::new(());
    
    /// Set environment variables for the duration of a test
    pub fn with_env_vars<F, R>(vars: HashMap<&str, &str>, test_fn: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _lock = ENV_MUTEX.lock().unwrap();
        
        // Store original values
        let mut originals = HashMap::new();
        for (key, value) in &vars {
            originals.insert(*key, std::env::var(key).ok());
            std::env::set_var(key, value);
        }
        
        // Run test
        let result = test_fn();
        
        // Restore original values
        for (key, original) in originals {
            match original {
                Some(val) => std::env::set_var(key, val),
                None => std::env::remove_var(key),
            }
        }
        
        result
    }
}

/// Mock service factories
pub mod mocks {
    use std::sync::Arc;
    use oxidizedoasis_websands::core::user::MockUserRepositoryTrait;
    use oxidizedoasis_websands::core::email::service::MockEmailServiceTrait;
    use oxidizedoasis_websands::core::auth::token_revocation::MockTokenRevocationServiceTrait;
    use oxidizedoasis_websands::core::auth::active_token::MockActiveTokenServiceTrait;
    use mockall::predicate;
    
    /// Create a mock user repository with basic expectations
    pub fn create_mock_user_repository() -> MockUserRepositoryTrait {
        let mut mock = MockUserRepositoryTrait::new();
        
        // Set up default expectations that can be overridden
        mock.expect_find_by_id().returning(|_| Ok(None));
        mock.expect_find_by_username().returning(|_| Ok(None));
        mock.expect_find_user_by_email().returning(|_| Ok(None));
        
        mock
    }
    
    /// Create a mock email service with basic expectations
    pub fn create_mock_email_service() -> MockEmailServiceTrait {
        let mut mock = MockEmailServiceTrait::new();
        
        mock.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        
        mock
    }
    
    /// Create a mock token revocation service
    pub fn create_mock_token_revocation_service() -> MockTokenRevocationServiceTrait {
        let mut mock = MockTokenRevocationServiceTrait::new();
        
        mock.expect_is_token_revoked().returning(|_| Ok(false));
        mock.expect_revoke_token().returning(|_, _, _, _, _| Ok(()));
        mock.expect_revoke_all_user_tokens()
            .with(predicate::always(), predicate::always())
            .returning(|_, _| Ok(0));
        mock.expect_cleanup_expired_tokens().returning(|| Ok(0));
        
        mock
    }
    
    /// Create a mock active token service
    pub fn create_mock_active_token_service() -> MockActiveTokenServiceTrait {
        let mut mock = MockActiveTokenServiceTrait::new();
        
        mock.expect_record_token().returning(|_, _, _, _, _| Ok(()));
        mock.expect_get_active_token().returning(|_| {
            use uuid::Uuid;
            use chrono::Utc;
            use oxidizedoasis_websands::core::auth::active_token::ActiveToken;
            
            Ok(ActiveToken {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                jti: "test_jti".to_string(),
                token_type: "Access".to_string(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
                created_at: Utc::now(),
                device_info: None,
            })
        });
        mock.expect_remove_token().returning(|_| Ok(true));
        mock.expect_get_user_tokens().returning(|_| Ok(vec![]));
        mock.expect_remove_all_user_tokens().returning(|_| Ok(0));
        mock.expect_cleanup_expired_tokens().returning(|| Ok(0));
        
        mock
    }
}