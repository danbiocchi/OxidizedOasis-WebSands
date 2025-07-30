//! Common test utilities and infrastructure

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

/// Generate a unique database name for each test to prevent race conditions
fn generate_unique_database_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    use uuid::Uuid;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    // Get current thread ID and timestamp for uniqueness
    let thread_id = std::thread::current().id();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    // Get atomic counter to ensure absolute uniqueness
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    
    // Format thread ID to be database-name safe (remove non-alphanumeric chars and convert to lowercase)
    let thread_str = format!("{:?}", thread_id)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    
    // Add UUID to ensure absolute uniqueness even if timestamp collides
    let uuid_suffix = Uuid::new_v4().to_string().replace("-", "").chars().take(8).collect::<String>();
    
    // Create unique database name: test_oxidizedoasis_db_<thread>_<counter>_<timestamp>_<uuid>
    format!("test_oxidizedoasis_db_{}_{}_{}_{}", thread_str, counter, timestamp % 1_000_000, uuid_suffix)
}

/// Create test configuration for integration tests with unique database names
fn create_test_config() -> AppConfig {
    let unique_db_name = generate_unique_database_name();
    
    // Build database URLs with the unique database name
    let base_url = "postgres://dreamer@localhost:5432";
    let database_url = format!("{}/{}", base_url, unique_db_name);
    let su_database_url = format!("{}/{}", base_url, unique_db_name);
    
    // Set environment variables for this test's unique database
    std::env::set_var("DATABASE_URL", format!("postgres://oxidizedoasis:pfut940AqIcy(B-HV*@localhost/{}", unique_db_name));
    std::env::set_var("SU_DATABASE_URL", &su_database_url);
    std::env::set_var("DB_NAME", &unique_db_name);
    std::env::set_var("DB_USER", "oxidizedoasis");
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    std::env::set_var("JWT_AUDIENCE", TEST_AUDIENCE);
    std::env::set_var("JWT_ISSUER", TEST_ISSUER);
    
    println!("🔧 [create_test_config] Generated unique database: {}", unique_db_name);
    
    AppConfig {
        server: oxidizedoasis_websands::infrastructure::config::app_config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: "8080".to_string(),
        },
        database: oxidizedoasis_websands::infrastructure::config::app_config::DatabaseConfig {
            url: format!("postgres://oxidizedoasis:pfut940AqIcy(B-HV*@localhost/{}", unique_db_name),
            max_connections: 5,
        },
        jwt: oxidizedoasis_websands::infrastructure::config::app_config::JwtConfig {
            secret: TEST_JWT_SECRET.to_string(),
            audience: TEST_AUDIENCE.to_string(),
            issuer: TEST_ISSUER.to_string(),
        },
    }
}

/// Create test configuration for integration tests
pub fn create_test_app_config() -> AppConfig {
    create_test_config()
}

/// Create test configuration with cleanup support - returns config and database name
pub async fn create_test_config_with_cleanup() -> Result<(AppConfig, String), Box<dyn std::error::Error>> {
    let unique_db_name = generate_unique_database_name();
    
    // Build database URLs with the unique database name
    let base_url = "postgres://dreamer@localhost:5432";
    let su_database_url = format!("{}/{}", base_url, unique_db_name);
    
    // Set environment variables for this test's unique database
    std::env::set_var("DATABASE_URL", format!("postgres://oxidizedoasis:pfut940AqIcy(B-HV*@localhost/{}", unique_db_name));
    std::env::set_var("SU_DATABASE_URL", &su_database_url);
    std::env::set_var("DB_NAME", &unique_db_name);
    std::env::set_var("DB_USER", "oxidizedoasis");
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("JWT_SECRET", TEST_JWT_SECRET);
    std::env::set_var("JWT_AUDIENCE", TEST_AUDIENCE);
    std::env::set_var("JWT_ISSUER", TEST_ISSUER);
    
    println!("🔧 [create_test_config_with_cleanup] Generated unique database: {}", unique_db_name);
    
    let config = AppConfig {
        server: oxidizedoasis_websands::infrastructure::config::app_config::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: "8080".to_string(),
        },
        database: oxidizedoasis_websands::infrastructure::config::app_config::DatabaseConfig {
            url: format!("postgres://oxidizedoasis:pfut940AqIcy(B-HV*@localhost/{}", unique_db_name),
            max_connections: 5,
        },
        jwt: oxidizedoasis_websands::infrastructure::config::app_config::JwtConfig {
            secret: TEST_JWT_SECRET.to_string(),
            audience: TEST_AUDIENCE.to_string(),
            issuer: TEST_ISSUER.to_string(),
        },
    };
    
    // The create_pool function already handles database creation and migration setup
    let _pool = create_pool(&config).await?;
    
    Ok((config, unique_db_name))
}

/// Cleanup test database after test completion
pub async fn cleanup_test_database(db_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use sqlx::postgres::PgPoolOptions;
    
    // Connect to PostgreSQL server (not the test database)
    let base_url = "postgres://dreamer@localhost:5432/postgres";
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(base_url)
        .await?;
    
    // First, forcefully terminate any active connections to the database
    let terminate_query = format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
        db_name
    );
    let _ = sqlx::query(&terminate_query)
        .execute(&pool)
        .await; // Ignore errors here as connections might already be closed
    
    // Wait a brief moment for connections to close
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Drop the test database
    let drop_query = format!("DROP DATABASE IF EXISTS \"{}\"", db_name);
    sqlx::query(&drop_query)
        .execute(&pool)
        .await
        .map_err(|e| {
            eprintln!("⚠️  [cleanup_test_database] Failed to drop database {}: {}", db_name, e);
            e
        })?;
    
    println!("🧹 [cleanup_test_database] Successfully dropped database: {}", db_name);
    pool.close().await;
    
    Ok(())
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
    use oxidizedoasis_websands::core::user::repository::MockUserRepositoryTrait;
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

/// Unified test fixture that eliminates 75% code duplication
/// This provides a standardized testing infrastructure with both mock and real DB support
pub struct UnifiedTestFixture {
    pub config: AppConfig,
    pub db_name: String,
    pub db_pool: PgPool,
    pub test_user_id: Uuid,
    pub test_admin_id: Uuid,
    pub test_target_user_id: Uuid,
    pub test_user_token: String,
    pub test_admin_token: String,
}

impl UnifiedTestFixture {
    /// Create fixture with real database (preferred approach from the plan)
    pub async fn new_with_database() -> Self {
        let (config, db_name) = create_test_config_with_cleanup().await
            .expect("Failed to create test config with cleanup");
        
        // Create the database pool
        let db_pool = create_pool(&config).await
            .expect("Failed to create database pool");
        
        let test_user_id = Uuid::new_v4();
        let test_admin_id = Uuid::new_v4();
        let test_target_user_id = Uuid::new_v4();
        
        let test_user_token = generate_test_token(test_user_id, "user", 3600)
            .expect("Failed to generate user token");
        let test_admin_token = generate_test_token(test_admin_id, "admin", 3600)
            .expect("Failed to generate admin token");
        
        Self {
            config,
            db_name,
            db_pool,
            test_user_id,
            test_admin_id,
            test_target_user_id,
            test_user_token,
            test_admin_token,
        }
    }
    
    /// Create fixture with mocks (for specific test needs)
    pub async fn new_with_mocks() -> Self {
        let config = create_test_app_config();
        
        // Create a database pool even for mocks (needed by handlers)
        let db_pool = create_pool(&config).await
            .expect("Failed to create database pool for mocks");
        
        let test_user_id = Uuid::new_v4();
        let test_admin_id = Uuid::new_v4();
        let test_target_user_id = Uuid::new_v4();
        
        let test_user_token = generate_test_token(test_user_id, "user", 3600)
            .expect("Failed to generate user token");
        let test_admin_token = generate_test_token(test_admin_id, "admin", 3600)
            .expect("Failed to generate admin token");
        
        Self {
            config,
            db_name: "mock_database".to_string(),
            db_pool,
            test_user_id,
            test_admin_id,
            test_target_user_id,
            test_user_token,
            test_admin_token,
        }
    }
    
    /// Standardized cleanup
    pub async fn cleanup(&self) {
        if let Err(e) = cleanup_test_database(&self.db_name).await {
            eprintln!("Failed to cleanup test database {}: {}", self.db_name, e);
        }
    }
}

/// Create standardized mock services and auth service for testing
/// This eliminates 50+ lines of duplication per test
pub async fn create_standard_mock_services(
    test_user_id: Uuid,
    test_admin_id: Uuid,
) -> (
    Arc<oxidizedoasis_websands::core::auth::AuthService>,
    oxidizedoasis_websands::api::handlers::user_handler::UserHandler,
    Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>,
) {
    use std::sync::Arc;
    use oxidizedoasis_websands::core::auth::AuthService;
    use oxidizedoasis_websands::api::handlers::user_handler::create_handler;

    // Create mock services
    let mut user_repo = mocks::create_mock_user_repository();
    let email_service = Arc::new(mocks::create_mock_email_service());
    let token_revocation_service = Arc::new(mocks::create_mock_token_revocation_service());
    let active_token_service = Arc::new(mocks::create_mock_active_token_service());
    
    // Set up user repository expectations for test users
    let test_user = create_test_user(test_user_id, test_data::TEST_USER_USERNAME, test_data::TEST_USER_EMAIL, true, "user");
    let test_admin = create_test_user(test_admin_id, test_data::TEST_ADMIN_USERNAME, test_data::TEST_ADMIN_EMAIL, true, "admin");
    let test_target_user = create_test_user(Uuid::new_v4(), "targetuser", "target@example.com", true, "user");
    
    // Clone users for the find_by_id closure - CRITICAL: Use exact UUID values to fix lookup
    let test_user_for_closure = test_user.clone();
    let test_admin_for_closure = test_admin.clone();
    
    // CRITICAL FIX: Capture UUID values in closure scope to ensure exact matching
    let expected_user_id = test_user_id;
    let expected_admin_id = test_admin_id;
    println!("🔧 [create_standard_mock_services] Setting up mock repository with expected UUIDs:");
    println!("🔧 [create_standard_mock_services] expected_user_id: {}", expected_user_id);
    println!("🔧 [create_standard_mock_services] expected_admin_id: {}", expected_admin_id);
    
    user_repo.expect_find_by_id()
        .returning(move |id| {
            println!("🔍 DEBUG: Mock find_by_id called with ID: {}", id);
            println!("🔍 DEBUG: Expected test_user_id: {}", expected_user_id);
            println!("🔍 DEBUG: Expected test_admin_id: {}", expected_admin_id);
            
            if id == expected_user_id {
                println!("🔍 DEBUG: Returning test_user for ID: {}", id);
                Ok(Some(test_user_for_closure.clone()))
            } else if id == expected_admin_id {
                println!("🔍 DEBUG: Returning test_admin for ID: {}", id);
                Ok(Some(test_admin_for_closure.clone()))
            } else {
                println!("🔍 DEBUG: No user found for ID: {}", id);
                Ok(None)
            }
        });

    // CRITICAL FIX: Add find_by_username expectations for login authentication flow
    let test_user_for_username_closure = test_user.clone();
    let test_admin_for_username_closure = test_admin.clone();
    
    user_repo.expect_find_by_username()
        .returning(move |username| {
            println!("🔍 DEBUG: Mock find_by_username called with username: {}", username);
            
            if username == test_data::TEST_USER_USERNAME {
                println!("🔍 DEBUG: Returning test_user for username: {}", username);
                Ok(Some(test_user_for_username_closure.clone()))
            } else if username == test_data::TEST_ADMIN_USERNAME {
                println!("🔍 DEBUG: Returning test_admin for username: {}", username);
                Ok(Some(test_admin_for_username_closure.clone()))
            } else {
                println!("🔍 DEBUG: No user found for username: {}", username);
                Ok(None)
            }
        });

    // Mock find_all for listing users - returns 3 users as expected by tests
    let all_users = vec![test_user.clone(), test_admin.clone(), test_target_user.clone()];
    user_repo.expect_find_all()
        .returning(move || Ok(all_users.clone()));

    // Create shared repository for both auth service and admin endpoints
    let shared_user_repo: Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait> = Arc::new(user_repo);

    let auth_service = Arc::new(AuthService::new(
        shared_user_repo.clone(),
        TEST_JWT_SECRET.to_string(),
        TEST_AUDIENCE.to_string(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));

    // Create a dummy config for pool creation
    let config = create_test_app_config();
    let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&config)
        .await
        .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e));

    let user_handler = create_handler(
        pool,
        email_service.clone(),
        auth_service.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
    );

    (auth_service, user_handler, token_revocation_service)
}

/// Create standardized mock services including UserRepository for admin user management tests
/// This eliminates 50+ lines of duplication per test and provides the UserRepository needed for admin endpoints
pub async fn create_standard_mock_services_with_repo(
    test_user_id: Uuid,
    test_admin_id: Uuid,
    test_target_user_id: Uuid,
) -> (
    Arc<oxidizedoasis_websands::core::auth::AuthService>,
    oxidizedoasis_websands::api::handlers::user_handler::UserHandler,
    Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait>,
) {
    use std::sync::Arc;
    use oxidizedoasis_websands::core::auth::AuthService;
    use oxidizedoasis_websands::api::handlers::user_handler::create_handler;

    // Create mock services
    let mut user_repo = mocks::create_mock_user_repository();
    let email_service = Arc::new(mocks::create_mock_email_service());
    let token_revocation_service = Arc::new(mocks::create_mock_token_revocation_service());
    let active_token_service = Arc::new(mocks::create_mock_active_token_service());
    
    // Set up user repository expectations for test users
    let test_user = create_test_user(test_user_id, test_data::TEST_USER_USERNAME, test_data::TEST_USER_EMAIL, true, "user");
    let test_admin = create_test_user(test_admin_id, test_data::TEST_ADMIN_USERNAME, test_data::TEST_ADMIN_EMAIL, true, "admin");
    let test_target_user = create_test_user(test_target_user_id, "targetuser", "target@example.com", true, "user");
    
    // Clone users for the find_by_id closure
    let test_user_for_closure = test_user.clone();
    let test_admin_for_closure = test_admin.clone();
    let test_target_user_for_closure = test_target_user.clone();
    
    // CRITICAL FIX: Capture UUID values in closure scope to ensure exact matching
    let expected_user_id = test_user_id;
    let expected_admin_id = test_admin_id;
    let expected_target_user_id = test_target_user_id;
    user_repo.expect_find_by_id()
        .returning(move |id| {
            if id == expected_user_id {
                Ok(Some(test_user_for_closure.clone()))
            } else if id == expected_admin_id {
                Ok(Some(test_admin_for_closure.clone()))
            } else if id == expected_target_user_id {
                Ok(Some(test_target_user_for_closure.clone()))
            } else {
                Ok(None)
            }
        });

    // CRITICAL FIX: Add find_by_username expectations for login authentication flow
    let test_user_for_username_closure = test_user.clone();
    let test_admin_for_username_closure = test_admin.clone();
    
    user_repo.expect_find_by_username()
        .returning(move |username| {
            if username == test_data::TEST_USER_USERNAME {
                Ok(Some(test_user_for_username_closure.clone()))
            } else if username == test_data::TEST_ADMIN_USERNAME {
                Ok(Some(test_admin_for_username_closure.clone()))
            } else {
                Ok(None)
            }
        });

    // Mock find_all for listing users - returns 3 users as expected by tests
    let all_users = vec![test_user.clone(), test_admin.clone(), test_target_user.clone()];
    user_repo.expect_find_all()
        .returning(move || Ok(all_users.clone()));

    // Add CRUD operation expectations for admin tests
    // Use the test_target_user_id parameter passed to this function
    
    // Clone users for different operation closures
    let test_target_user_for_update_role = test_target_user.clone();
    let test_target_user_for_update_username = test_target_user.clone();
    let test_target_user_for_update_status = test_target_user.clone();
    
    // Mock update_role - returns updated user with new role
    let target_id_for_update_role = test_target_user_id;
    user_repo.expect_update_role()
        .returning(move |id, role| {
            if id == target_id_for_update_role {
                let mut updated_user = test_target_user_for_update_role.clone();
                updated_user.role = role.to_string();
                Ok(Some(updated_user))
            } else {
                Ok(None)
            }
        });

    // Mock update_username - returns updated user with new username
    let target_id_for_update_username = test_target_user_id;
    user_repo.expect_update_username()
        .returning(move |id, username| {
            if id == target_id_for_update_username {
                let mut updated_user = test_target_user_for_update_username.clone();
                updated_user.username = username.to_string();
                Ok(Some(updated_user))
            } else {
                Ok(None)
            }
        });

    // Mock update_status - returns updated user with new status
    let target_id_for_update_status = test_target_user_id;
    user_repo.expect_update_status()
        .returning(move |id, is_active| {
            if id == target_id_for_update_status {
                let mut updated_user = test_target_user_for_update_status.clone();
                updated_user.is_active = is_active;
                Ok(Some(updated_user))
            } else {
                Ok(None)
            }
        });

    // Mock delete - returns true for test_target_user_id, false otherwise
    let target_id_for_delete = test_target_user_id;
    user_repo.expect_delete()
        .returning(move |id| {
            if id == target_id_for_delete {
                Ok(true)
            } else {
                Ok(false)
            }
        });

    // Create shared repository for both auth service and admin endpoints
    let shared_user_repo: Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait> = Arc::new(user_repo);

    let auth_service = Arc::new(AuthService::new(
        shared_user_repo.clone(),
        TEST_JWT_SECRET.to_string(),
        TEST_AUDIENCE.to_string(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));

    // Create a dummy config for pool creation
    let config = create_test_app_config();
    let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&config)
        .await
        .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e));

    let user_handler = create_handler(
        pool,
        email_service.clone(),
        auth_service.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
    );

    (auth_service, user_handler, shared_user_repo)
}

/// Create authenticated request helper
pub fn create_authenticated_request(method: &str, uri: &str, token: &str) -> actix_web::test::TestRequest {
    http::create_auth_request(method, uri, token)
}

// Enhanced Test Utilities for Phase 3 Migration
use serde_json::Value;

/// Enhanced TestConfig with integration/unit test optimizations
pub struct EnhancedTestConfig {
    pub pool: Arc<PgPool>,
    pub config: AppConfig,
    pub use_real_database: bool,
}

impl EnhancedTestConfig {
    /// Create test config optimized for unit tests with mocked dependencies
    pub async fn new_for_unit_tests() -> Self {
        let config = create_test_app_config();
        let pool = Arc::new(create_pool(&config).await.expect("Failed to create pool"));
        
        Self {
            pool,
            config,
            use_real_database: false,
        }
    }

    /// Create test config optimized for integration tests with real database
    pub async fn new_for_integration_tests() -> Self {
        let (config, _db_name) = create_test_config_with_cleanup().await
            .expect("Failed to create test config with cleanup");
        let pool = Arc::new(create_pool(&config).await.expect("Failed to create pool"));
        
        Self {
            pool,
            config,
            use_real_database: true,
        }
    }

    pub async fn cleanup(&self) {
        // Enhanced cleanup for both mock and real database scenarios
        let _ = sqlx::query("DELETE FROM users WHERE email LIKE '%test%'")
            .execute(self.pool.as_ref())
            .await;
        let _ = sqlx::query("DELETE FROM password_resets WHERE email LIKE '%test%'")
            .execute(self.pool.as_ref())
            .await;
        let _ = sqlx::query("DELETE FROM revoked_tokens")
            .execute(self.pool.as_ref())
            .await;
        let _ = sqlx::query("DELETE FROM active_tokens")
            .execute(self.pool.as_ref())
            .await;
    }
}

// Enhanced Assertion Helpers
/// Assert user response contains expected data
pub fn assert_user_response(response: &Value, expected_email: &str, expected_role: &str) {
    assert_eq!(response["success"], true);
    assert_eq!(response["data"]["email"], expected_email);
    assert_eq!(response["data"]["role"], expected_role);
    assert!(response["data"]["id"].is_string());
    assert!(response["data"]["created_at"].is_string() || response["data"]["created_at"].is_null());
}

/// Assert error response contains expected status and message
pub fn assert_error_response(response: &Value, expected_message_contains: &str) {
    // For API error responses, check message field
    if let Some(message) = response["message"].as_str() {
        assert!(message.contains(expected_message_contains),
                "Expected message to contain '{}', but got '{}'", expected_message_contains, message);
    } else {
        panic!("Expected error response to have 'message' field");
    }
}

/// Assert user exists in database with expected properties
pub async fn assert_user_in_database(pool: &PgPool, user_id: Uuid, expected_email: &str) {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, username, email, password_hash, role, is_active, is_email_verified,
                  created_at, updated_at, verification_token, verification_token_expires_at
           FROM users WHERE id = $1"#,
        user_id
    )
    .fetch_one(pool)
    .await
    .expect("User should exist in database");

    assert_eq!(user.email.unwrap_or_default(), expected_email);
    assert_eq!(user.id, user_id);
}

// Data Seeding Utilities
/// Seed multiple user scenarios for comprehensive testing
pub async fn seed_user_scenarios(pool: &PgPool) -> Vec<User> {
    let mut users = Vec::new();
    
    // Regular user
    let regular_user = create_test_db_user(pool, "regular@test.com", "user", true).await;
    users.push(regular_user);
    
    // Admin user
    let admin_user = create_test_db_user(pool, "admin@test.com", "admin", true).await;
    users.push(admin_user);
    
    // Unverified user
    let unverified_user = create_test_db_user(pool, "unverified@test.com", "user", false).await;
    users.push(unverified_user);
    
    users
}

/// Seed admin-specific test data
pub async fn seed_admin_test_data(pool: &PgPool) -> (User, User) {
    let admin = create_test_db_user(pool, "test-admin@test.com", "admin", true).await;
    let target_user = create_test_db_user(pool, "target-user@test.com", "user", true).await;
    (admin, target_user)
}

/// Seed workflow test data for complex scenarios
pub async fn seed_workflow_test_data(pool: &PgPool) -> Vec<User> {
    let mut users = Vec::new();
    
    // Create multiple users for complex workflow testing
    for i in 1..=5 {
        let email = format!("workflow-user-{}@test.com", i);
        let role = if i == 1 { "admin" } else { "user" };
        let user = create_test_db_user(pool, &email, role, true).await;
        users.push(user);
    }
    
    users
}

/// Helper function to create a user in the database
async fn create_test_db_user(pool: &PgPool, email: &str, role: &str, email_verified: bool) -> User {
    let password_hash = bcrypt::hash("test_password", bcrypt::DEFAULT_COST).unwrap();
    let username = email.split('@').next().unwrap().to_string();
    let user_id = Uuid::new_v4();
    
    sqlx::query_as!(
        User,
        r#"INSERT INTO users (id, username, email, password_hash, role, is_email_verified)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, username, email, password_hash, role, is_active, is_email_verified,
                     created_at, updated_at, verification_token, verification_token_expires_at"#,
        user_id,
        username,
        email,
        password_hash,
        role,
        email_verified
    )
    .fetch_one(pool)
    .await
    .expect("Failed to create test user")
}

// User Management Scenario Builder
/// Scenario builder for user management workflows
pub struct UserManagementScenario {
    pub config: EnhancedTestConfig,
    pub admin_user: Option<User>,
    pub target_users: Vec<User>,
}

impl UserManagementScenario {
    /// Create new scenario with integration test configuration
    pub async fn new() -> Self {
        let config = EnhancedTestConfig::new_for_integration_tests().await;
        Self {
            config,
            admin_user: None,
            target_users: Vec::new(),
        }
    }

    /// Add admin user to scenario
    pub async fn with_admin_user(mut self) -> Self {
        self.admin_user = Some(create_test_db_user(&self.config.pool, "scenario-admin@test.com", "admin", true).await);
        self
    }

    /// Add multiple target users to scenario
    pub async fn with_target_users(mut self, count: usize) -> Self {
        for i in 1..=count {
            let email = format!("scenario-target-{}@test.com", i);
            let user = create_test_db_user(&self.config.pool, &email, "user", true).await;
            self.target_users.push(user);
        }
        self
    }

    /// Get admin user reference
    pub fn admin(&self) -> &User {
        self.admin_user.as_ref().expect("Admin user not set")
    }

    /// Get target user by index
    pub fn target_user(&self, index: usize) -> &User {
        &self.target_users[index]
    }

    /// Cleanup scenario
    pub async fn cleanup(self) {
        self.config.cleanup().await;
    }
}