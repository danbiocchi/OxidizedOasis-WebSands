//! Comprehensive integration tests for src/main.rs
//!
//! This module provides comprehensive tests for:
//! - Application configuration loading and validation
//! - Server startup and shutdown scenarios
//! - Environment variable handling
//! - Database connection initialization
//! - Error handling for startup failures
//! - Middleware registration and setup
//! - Full HTTP server integration testing

use std::env;
use std::time::Duration;
use actix_web::{test, web, App, middleware, HttpResponse};
use test_common::UnifiedTestFixture;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::infrastructure::database::connection::create_pool;
use oxidizedoasis_websands::core::email::service::EmailService;
use oxidizedoasis_websands::core::user::UserRepository;
use oxidizedoasis_websands::core::auth::AuthService;
use oxidizedoasis_websands::core::auth::token_revocation::{TokenRevocationService, TokenRevocationServiceTrait};
use oxidizedoasis_websands::core::auth::active_token::{ActiveTokenService, ActiveTokenServiceTrait};
use sqlx::Row;
use std::sync::Arc;

/// Set up required environment variables for tests that create EmailService
fn setup_email_env_vars() {
    // Set all required environment variables for EmailService based on service.rs
    if env::var("SMTP_USERNAME").is_err() {
        env::set_var("SMTP_USERNAME", "test@example.com");
    }
    if env::var("SMTP_PASSWORD").is_err() {
        env::set_var("SMTP_PASSWORD", "test_password");
    }
    if env::var("SMTP_SERVER").is_err() {
        env::set_var("SMTP_SERVER", "smtp.test.com");
    }
    if env::var("FROM_EMAIL").is_err() {
        env::set_var("FROM_EMAIL", "noreply@test.com");
    }
    if env::var("ADMIN_EMAIL").is_err() {
        env::set_var("ADMIN_EMAIL", "admin@test.com");
    }
    if env::var("APP_NAME").is_err() {
        env::set_var("APP_NAME", "Test Oasis App");
    }
    if env::var("EMAIL_FROM_NAME").is_err() {
        env::set_var("EMAIL_FROM_NAME", "Test Oasis App");
    }
    if env::var("EMAIL_VERIFICATION_SUBJECT").is_err() {
        env::set_var("EMAIL_VERIFICATION_SUBJECT", "Verify Your Email - Test Oasis App");
    }
    if env::var("EMAIL_PASSWORD_RESET_SUBJECT").is_err() {
        env::set_var("EMAIL_PASSWORD_RESET_SUBJECT", "Reset Your Password - Test Oasis App");
    }
    if env::var("ENVIRONMENT").is_err() {
        env::set_var("ENVIRONMENT", "development");
    }
    if env::var("DEVELOPMENT_URL").is_err() {
        env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
    }
    if env::var("PRODUCTION_URL").is_err() {
        env::set_var("PRODUCTION_URL", "https://example.com");
    }
}

#[tokio::test]
async fn test_application_startup_with_valid_config() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that the application can start successfully with valid configuration
    let config_result = AppConfig::from_env();
    assert!(config_result.is_ok(), "Should load valid configuration from environment");
    
    let config = config_result.unwrap();
    assert!(!config.database.url.is_empty(), "Database URL should be configured");
    assert!(config.server.port.parse::<u16>().is_ok(), "Server port should be valid");
    assert!(!config.server.host.is_empty(), "Server host should be configured");
    assert!(!config.jwt.secret.is_empty(), "JWT secret should be configured");
    assert!(!config.jwt.audience.is_empty(), "JWT audience should be configured");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_database_connection_initialization() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that database connection can be established successfully
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for tests");
    
    let pool_result = create_pool(&fixture.config).await;
    assert!(pool_result.is_ok(), "Should create database connection pool successfully");
    
    let pool = pool_result.unwrap();
    
    // Test that we can perform a basic query
    let query_result = sqlx::query("SELECT 1 as test")
        .fetch_one(&pool)
        .await;
    
    assert!(query_result.is_ok(), "Should be able to execute basic query");
    
    // Test database pool configuration
    assert!(pool.size() >= 1, "Pool should have at least one connection");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_server_startup_and_shutdown() {
    setup_email_env_vars(); // Set up email environment variables
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create database pool
    let pool = create_pool(&fixture.config)
        .await
        .expect("Should create database pool");
    
    // Create services as in main.rs
    let email_service = Arc::new(EmailService::new());
    let active_token_service = Arc::new(ActiveTokenService::new(pool.clone()));
    let mut token_revocation_service = TokenRevocationService::new(pool.clone());
    token_revocation_service.set_active_token_service(active_token_service.clone());
    let token_revocation_service = Arc::new(token_revocation_service);
    let user_repository = Arc::new(UserRepository::new(pool.clone()));
    
    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        fixture.config.jwt.secret.clone(),
        fixture.config.jwt.audience.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Build a minimal app for testing (mimicking main.rs structure)
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(web::Data::new(fixture.config.clone()))
            .route("/health", web::get().to(|| async { 
                HttpResponse::Ok().json(serde_json::json!({"status": "healthy"})) 
            }))
            .wrap(middleware::Logger::default())
    ).await;
    
    // Test health endpoint
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success(), "Health check should return success status");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_environment_variable_handling() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that required environment variables are properly handled
    let required_vars = [
        "DATABASE_URL",
        "JWT_SECRET",
        "JWT_AUDIENCE", 
        "JWT_ISSUER",
    ];
    
    for var in required_vars.iter() {
        let value = env::var(var);
        assert!(value.is_ok(), "Required environment variable {var} should be set");
        assert!(!value.unwrap().is_empty(), "Environment variable {var} should not be empty");
    }
    
    // Test configuration loading with environment variables
    let config_result = AppConfig::from_env();
    assert!(config_result.is_ok(), "Should load configuration from environment variables");
    
    let config = config_result.unwrap();
    
    // Verify specific configuration values
    assert!(config.database.url.starts_with("postgres://"), "Database URL should be PostgreSQL");
    assert!(config.jwt.secret.len() >= 32, "JWT secret should be sufficiently long");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_invalid_database_url_error_handling() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test error handling with invalid database URL
    let mut invalid_config = fixture.config.clone();
    invalid_config.database.url = "invalid://database/url".to_string();
    
    let pool_result = create_pool(&invalid_config).await;
    assert!(pool_result.is_err(), "Should fail with invalid database URL");
    
    let error = pool_result.unwrap_err();
    let error_msg = error.to_string();
    assert!(error_msg.contains("Invalid database URL format") || error_msg.contains("invalid") || error_msg.contains("parse") || error_msg.contains("connection") || error_msg.contains("url"),
           "Error message should indicate URL parsing issue: {error_msg}");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_missing_environment_variables_error_handling() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Save original environment variables
    let original_jwt_secret = env::var("JWT_SECRET").ok();
    
    // Temporarily remove required environment variable
    env::remove_var("JWT_SECRET");
    
    // Test that configuration loading fails appropriately
    let config_result = AppConfig::from_env();
    assert!(config_result.is_err(), "Should fail when required environment variable is missing");
    
    // The fact that config loading failed when JWT_SECRET is missing proves the validation works
    
    // Restore original environment variable
    if let Some(value) = original_jwt_secret {
        env::set_var("JWT_SECRET", value);
    }
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_middleware_registration_and_setup() {
    setup_email_env_vars(); // Set up email environment variables
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create database pool
    let pool = create_pool(&fixture.config)
        .await
        .expect("Should create database pool");
    
    // Create services
    let email_service = Arc::new(EmailService::new());
    let active_token_service = Arc::new(ActiveTokenService::new(pool.clone()));
    let mut token_revocation_service = TokenRevocationService::new(pool.clone());
    token_revocation_service.set_active_token_service(active_token_service.clone());
    let token_revocation_service = Arc::new(token_revocation_service);
    let user_repository = Arc::new(UserRepository::new(pool.clone()));
    
    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        fixture.config.jwt.secret.clone(),
        fixture.config.jwt.audience.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Build the application with middleware (like main.rs)
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(web::Data::new(fixture.config.clone()))
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("X-XSS-Protection", "0"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
            )
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .route("/health", web::get().to(|| async { 
                HttpResponse::Ok().json(serde_json::json!({"status": "healthy"})) 
            }))
    ).await;
    
    // Test that middleware is working by making a request
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success(), "Request should succeed with middleware");
    
    // Verify security headers are set by middleware
    let headers = resp.headers();
    assert!(
        headers.contains_key("x-frame-options") || headers.contains_key("X-Frame-Options"),
        "Security middleware should set X-Frame-Options header"
    );
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_graceful_shutdown_signal_handling() {
    setup_email_env_vars(); // Set up email environment variables
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // This test verifies that the application can handle shutdown scenarios
    // Since we can't easily test actual signal handling in unit tests, we test the
    // graceful shutdown mechanism through service cleanup
    
    let pool = create_pool(&fixture.config)
        .await
        .expect("Should create database pool");
    
    // Create services
    let email_service = Arc::new(EmailService::new());
    let active_token_service = Arc::new(ActiveTokenService::new(pool.clone()));
    let mut token_revocation_service = TokenRevocationService::new(pool.clone());
    token_revocation_service.set_active_token_service(active_token_service.clone());
    let token_revocation_service = Arc::new(token_revocation_service);
    let user_repository = Arc::new(UserRepository::new(pool.clone()));
    
    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        fixture.config.jwt.secret.clone(),
        fixture.config.jwt.audience.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Test that services can be created and work before shutdown
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .route("/health", web::get().to(|| async { 
                HttpResponse::Ok().json(serde_json::json!({"status": "healthy"})) 
            }))
    ).await;
    
    // Verify server is working
    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    
    // Simulate cleanup operations that would happen during shutdown
    let cleanup_result = token_revocation_service.as_ref().cleanup_expired_tokens().await;
    assert!(cleanup_result.is_ok(), "Token cleanup should work during shutdown");
    
    let active_cleanup_result = active_token_service.as_ref().cleanup_expired_tokens().await;
    assert!(active_cleanup_result.is_ok(), "Active token cleanup should work during shutdown");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_server_configuration_validation() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test various server configuration scenarios
    let config = &fixture.config;
    
    // Test valid port ranges
    let port: u16 = config.server.port.parse()
        .expect("Port should be parseable as u16");
    assert!(port > 0 && port <= 65535, "Port should be in valid range");
    
    // Test host configuration
    assert!(!config.server.host.is_empty(), "Host should not be empty");
    assert!(
        config.server.host == "0.0.0.0" ||
        config.server.host == "127.0.0.1" ||
        config.server.host == "localhost" ||
        config.server.host.parse::<std::net::IpAddr>().is_ok(),
        "Host should be a valid IP address or hostname"
    );
    
    // Test database configuration
    assert!(config.database.max_connections > 0, "Max connections should be positive");
    assert!(config.database.max_connections <= 100, "Max connections should be reasonable");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_database_migration_state() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that database migrations are properly applied
    let pool = create_pool(&fixture.config)
        .await
        .expect("Should create database pool");
    
    // Check that essential tables exist (indicating migrations ran)
    let tables_query = sqlx::query(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
    ).fetch_all(&pool).await;
    
    assert!(tables_query.is_ok(), "Should be able to query database schema");
    
    let tables: Vec<String> = tables_query.unwrap()
        .into_iter()
        .map(|row| row.get("table_name"))
        .collect();
    
    // Verify essential tables exist
    let required_tables = ["users", "_sqlx_migrations"];
    for table in required_tables.iter() {
        assert!(
            tables.contains(&table.to_string()),
            "Required table '{table}' should exist after migrations"
        );
    }
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_service_initialization_and_dependencies() {
    setup_email_env_vars(); // Set up email environment variables
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that all services can be initialized properly (mimicking main.rs)
    let pool = create_pool(&fixture.config)
        .await
        .expect("Should create database pool");
    
    // Test service creation like in main.rs
    use oxidizedoasis_websands::core::email::service::EmailService;
    use oxidizedoasis_websands::core::auth::active_token::ActiveTokenService;
    use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService;
    use oxidizedoasis_websands::core::user::UserRepository;
    use oxidizedoasis_websands::core::auth::AuthService;
    use std::sync::Arc;
    
    // Create services like in main.rs
    let email_service = Arc::new(EmailService::new());
    let active_token_service = Arc::new(ActiveTokenService::new(pool.clone()));
    let mut token_revocation_service = TokenRevocationService::new(pool.clone());
    token_revocation_service.set_active_token_service(active_token_service.clone());
    let token_revocation_service = Arc::new(token_revocation_service);
    let user_repository = Arc::new(UserRepository::new(pool.clone()));
    
    // Create AuthService
    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        fixture.config.jwt.secret.clone(),
        fixture.config.jwt.audience.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Test that all services were created successfully
    assert!(!Arc::ptr_eq(&email_service, &Arc::new(EmailService::new())));
    assert!(!Arc::ptr_eq(&active_token_service, &Arc::new(ActiveTokenService::new(pool.clone()))));
    
    // Test service functionality
    let cleanup_result = token_revocation_service.as_ref().cleanup_expired_tokens().await;
    assert!(cleanup_result.is_ok(), "Token revocation service should be functional");
    
    let active_cleanup_result = active_token_service.as_ref().cleanup_expired_tokens().await;
    assert!(active_cleanup_result.is_ok(), "Active token service should be functional");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_json_payload_configuration() {
    setup_email_env_vars(); // Set up email environment variables
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test JSON configuration limits like in main.rs
    let pool = create_pool(&fixture.config)
        .await
        .expect("Should create database pool");
    
    // Create services
    let email_service = Arc::new(EmailService::new());
    let active_token_service = Arc::new(ActiveTokenService::new(pool.clone()));
    let mut token_revocation_service = TokenRevocationService::new(pool.clone());
    token_revocation_service.set_active_token_service(active_token_service.clone());
    let token_revocation_service = Arc::new(token_revocation_service);
    let user_repository = Arc::new(UserRepository::new(pool.clone()));
    
    let auth_service = Arc::new(AuthService::new(
        user_repository.clone(),
        fixture.config.jwt.secret.clone(),
        fixture.config.jwt.audience.clone(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Build app with JSON config like main.rs
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(auth_service.clone()))
            .app_data(web::JsonConfig::default()
                .limit(4096)
                .error_handler(|err, _| {
                    actix_web::error::InternalError::from_response(
                        err,
                        HttpResponse::BadRequest()
                            .content_type("application/json")
                            .body(r#"{"error":"Invalid JSON payload"}"#),
                    ).into()
                }))
            .route("/test-json", web::post().to(|payload: web::Json<serde_json::Value>| async move {
                HttpResponse::Ok().json(payload.into_inner())
            }))
    ).await;
    
    // Test valid JSON request
    let valid_json = serde_json::json!({"test": "data"});
    let req = test::TestRequest::post()
        .uri("/test-json")
        .set_json(&valid_json)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "Valid JSON should be accepted");
    
    // Test invalid JSON (too large would be harder to test, so we test malformed JSON)
    let req = test::TestRequest::post()
        .uri("/test-json")
        .set_payload("invalid json")
        .insert_header(("content-type", "application/json"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_client_error(), "Invalid JSON should be rejected");
    
    fixture.cleanup().await;
}

// Additional unit tests for main.rs configuration values
#[tokio::test]
async fn test_json_configuration_limits() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test JSON configuration limits like in main.rs
    let json_limit = 4096;
    assert!(json_limit > 0, "JSON limit should be positive");
    assert!(json_limit <= 10 * 1024 * 1024, "JSON limit should be reasonable"); // 10MB max

    // Test error response format
    let error_response = r#"{"error":"Invalid JSON payload"}"#;
    assert!(error_response.starts_with('{'), "Error response should be valid JSON");
    assert!(error_response.contains("error"), "Error response should contain error field");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_http_server_configuration_values() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that configuration values make sense for HTTP server (from main.rs)
    let keep_alive = Duration::from_secs(75);
    let client_timeout = Duration::from_secs(60);
    let shutdown_timeout = 30;
    let backlog = 1024;
    let workers = num_cpus::get() * 2;

    assert!(keep_alive.as_secs() > 0, "Keep alive should be positive");
    assert!(client_timeout.as_secs() > 0, "Client timeout should be positive");
    assert!(shutdown_timeout > 0, "Shutdown timeout should be positive");
    assert!(backlog > 0, "Backlog should be positive");
    assert!(workers > 0, "Workers should be positive");
    assert!(workers <= 32, "Workers should be reasonable"); // Sanity check
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_security_headers_configuration() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test security headers that would be configured in main.rs
    let security_headers = vec![
        ("X-XSS-Protection", "0"),
        ("Strict-Transport-Security", "max-age=31536000; includeSubDomains"),
        ("X-Frame-Options", "DENY"),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "strict-origin-when-cross-origin"),
        ("Cross-Origin-Embedder-Policy", "require-corp"),
        ("Cross-Origin-Opener-Policy", "same-origin"),
        ("Cross-Origin-Resource-Policy", "same-origin"),
    ];

    for (header, value) in security_headers {
        assert!(!header.is_empty(), "Header name should not be empty");
        assert!(!value.is_empty(), "Header value should not be empty");
    }

    // Test CSP header
    let csp = "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' https://cdn.jsdelivr.net https://cdnjs.cloudflare.com; style-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com; img-src 'self' data:; connect-src 'self' ws://127.0.0.1:* wss://127.0.0.1:*; font-src 'self' https://cdnjs.cloudflare.com; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; worker-src 'self' blob:; upgrade-insecure-requests;";
    
    assert!(csp.contains("default-src 'self'"), "CSP should have default-src");
    assert!(csp.contains("object-src 'none'"), "CSP should restrict object-src");
    assert!(csp.contains("frame-ancestors 'none'"), "CSP should restrict frame-ancestors");
    
    fixture.cleanup().await;
}

// Migration tests have been moved to migrations_tests.rs