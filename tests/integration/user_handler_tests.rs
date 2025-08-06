//! Integration tests for src/api/handlers/user_handler.rs
//!
//! This module provides integration tests for UserHandler functionality.

use std::sync::Arc;
use test_common::UnifiedTestFixture;
use oxidizedoasis_websands::api::handlers::user_handler::create_handler;
use oxidizedoasis_websands::core::user::UserRepository;
use oxidizedoasis_websands::core::auth::AuthService;
use oxidizedoasis_websands::core::email::service::MockEmailServiceTrait;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService;
use oxidizedoasis_websands::core::auth::active_token::ActiveTokenService;

/// Test UserHandler creation with mock email service
#[tokio::test]
async fn test_user_handler_creation() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create mock email service
    let email_service = Arc::new(MockEmailServiceTrait::new());
    let token_revocation_service = Arc::new(TokenRevocationService::new(fixture.db_pool.clone()));
    let active_token_service = Arc::new(ActiveTokenService::new(fixture.db_pool.clone()));
    
    let user_repo = Arc::new(UserRepository::new(fixture.db_pool.clone()));
    let auth_service = Arc::new(AuthService::new(
        user_repo,
        "test_secret_key_for_testing_12345".to_string(),
        "test_audience".to_string(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Test UserHandler creation
    let _handler = create_handler(
        fixture.db_pool.clone(),
        email_service,
        auth_service,
        token_revocation_service,
        active_token_service,
    );
    
    // If we reach here, creation was successful
    println!("UserHandler created successfully");
    
    fixture.cleanup().await;
}

/// Test UserHandler service integration
#[tokio::test]
async fn test_user_handler_service_integration() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create mock email service that succeeds
    let mut email_service = MockEmailServiceTrait::new();
    email_service.expect_send_verification_email()
        .returning(|_, _| Ok(()));
    email_service.expect_send_password_reset_email()
        .returning(|_, _| Ok(()));
    
    let email_service = Arc::new(email_service);
    let token_revocation_service = Arc::new(TokenRevocationService::new(fixture.db_pool.clone()));
    let active_token_service = Arc::new(ActiveTokenService::new(fixture.db_pool.clone()));
    
    let user_repo = Arc::new(UserRepository::new(fixture.db_pool.clone()));
    let auth_service = Arc::new(AuthService::new(
        user_repo.clone(),
        "test_secret_key_for_testing_12345".to_string(),
        "test_audience".to_string(),
        token_revocation_service.clone(),
        active_token_service.clone(),
        email_service.clone(),
    ));
    
    // Test UserHandler creation with configured services
    let _handler = create_handler(
        fixture.db_pool.clone(),
        email_service,
        auth_service,
        token_revocation_service,
        active_token_service,
    );
    
    // Test basic database connectivity through the repository
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&fixture.db_pool)
        .await;
    assert!(user_count.is_ok(), "Should be able to query users table");
    
    println!("UserHandler service integration test passed");
    
    fixture.cleanup().await;
}