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