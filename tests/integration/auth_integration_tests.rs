//! Integration tests for src/core/auth/token_revocation.rs
//!
//! This module provides comprehensive integration tests for:
//! - Token revocation functionality 
//! - Multi-token user scenarios
//! - Active token service integration
//! - Error handling paths
//! - Database interaction validation

use std::sync::Arc;
use uuid::Uuid;
use chrono::{Utc, Duration};
use test_common::UnifiedTestFixture;
use oxidizedoasis_websands::core::auth::token_revocation::{TokenRevocationService, TokenRevocationServiceTrait};
use oxidizedoasis_websands::core::auth::active_token::{ActiveTokenService, ActiveTokenServiceTrait, ActiveToken};
use oxidizedoasis_websands::core::auth::jwt::TokenType;

/// Helper function to create a real user in the database for testing
async fn create_test_db_user(fixture: &UnifiedTestFixture) -> Uuid {
    let user_id = Uuid::new_v4();
    let query = r#"
        INSERT INTO users (id, username, email, password_hash, role, created_at, updated_at, is_email_verified)
        VALUES ($1, $2, $3, $4, 'user', NOW(), NOW(), true)
    "#;
    
    sqlx::query(query)
        .bind(user_id)
        .bind(format!("testuser_{}", user_id))
        .bind(format!("test_{}@example.com", user_id))
        .bind("$2b$12$dummy_hash") // Dummy bcrypt hash
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");
    
    user_id
}

#[tokio::test]
async fn test_revoke_all_user_tokens_with_multiple_tokens() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create services
    let active_token_service = Arc::new(ActiveTokenService::new(fixture.db_pool.clone()));
    let mut token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    token_revocation_service.set_active_token_service(active_token_service.clone());
    
    let user_id = create_test_db_user(&fixture).await;
    let expires_at = Utc::now() + Duration::hours(1);
    
    // Create multiple active tokens for the user
    let token1_jti = "test_token_1".to_string();
    let token2_jti = "test_token_2".to_string();
    let token3_jti = "test_token_3".to_string();
    
    // Record active tokens
    let record_result1 = active_token_service.record_token(
        user_id,
        &token1_jti,
        TokenType::Access,
        expires_at,
        None,
    ).await;
    assert!(record_result1.is_ok(), "Should record first token successfully");
    
    let record_result2 = active_token_service.record_token(
        user_id,
        &token2_jti,
        TokenType::Refresh,
        expires_at,
        None,
    ).await;
    assert!(record_result2.is_ok(), "Should record second token successfully");
    
    let record_result3 = active_token_service.record_token(
        user_id,
        &token3_jti,
        TokenType::Access,
        expires_at,
        None,
    ).await;
    assert!(record_result3.is_ok(), "Should record third token successfully");
    
    // Verify tokens exist before revocation
    let tokens_before = active_token_service.get_user_tokens(user_id).await;
    assert!(tokens_before.is_ok(), "Should get user tokens");
    assert_eq!(tokens_before.unwrap().len(), 3, "Should have 3 active tokens");
    
    // Revoke all user tokens
    let revoke_result = token_revocation_service.revoke_all_user_tokens(user_id, Some("Test revocation")).await;
    assert!(revoke_result.is_ok(), "Should revoke all tokens successfully");
    assert_eq!(revoke_result.unwrap(), 3, "Should have revoked 3 tokens");
    
    // Verify tokens are revoked
    let is_revoked1 = token_revocation_service.is_token_revoked(&token1_jti).await;
    assert!(is_revoked1.is_ok() && is_revoked1.unwrap(), "First token should be revoked");
    
    let is_revoked2 = token_revocation_service.is_token_revoked(&token2_jti).await;
    assert!(is_revoked2.is_ok() && is_revoked2.unwrap(), "Second token should be revoked");
    
    let is_revoked3 = token_revocation_service.is_token_revoked(&token3_jti).await;
    assert!(is_revoked3.is_ok() && is_revoked3.unwrap(), "Third token should be revoked");
    
    // Verify active tokens are removed
    let tokens_after = active_token_service.get_user_tokens(user_id).await;
    assert!(tokens_after.is_ok(), "Should get user tokens after revocation");
    assert_eq!(tokens_after.unwrap().len(), 0, "Should have no active tokens after revocation");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_revoke_all_user_tokens_with_get_user_tokens_error() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create mock active token service that returns error
    use oxidizedoasis_websands::core::auth::active_token::MockActiveTokenServiceTrait;
    use mockall::predicate;
    
    let mut mock_active_service = MockActiveTokenServiceTrait::new();
    mock_active_service
        .expect_get_user_tokens()
        .with(predicate::always())
        .returning(|_| Err(sqlx::Error::RowNotFound));
    
    let active_service_arc = Arc::new(mock_active_service);
    let mut token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    token_revocation_service.set_active_token_service(active_service_arc);
    
    let user_id = Uuid::new_v4();
    
    // Test that error is handled gracefully
    let revoke_result = token_revocation_service.revoke_all_user_tokens(user_id, Some("Test error handling")).await;
    assert!(revoke_result.is_ok(), "Should handle get_user_tokens error gracefully");
    assert_eq!(revoke_result.unwrap(), 0, "Should return 0 revoked tokens when get_user_tokens fails");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_revoke_all_user_tokens_with_no_active_token_service() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create token revocation service without setting active token service
    let token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    let user_id = Uuid::new_v4();
    
    // Test that it doesn't panic when active_token_service is None
    let revoke_result = token_revocation_service.revoke_all_user_tokens(user_id, Some("Test without active service")).await;
    assert!(revoke_result.is_ok(), "Should handle missing active token service gracefully");
    assert_eq!(revoke_result.unwrap(), 0, "Should return 0 revoked tokens when no active service");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_token_revocation_individual_operations() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    let token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    let user_id = create_test_db_user(&fixture).await;
    let jti = "individual_test_token";
    let expires_at = Utc::now() + Duration::hours(1);
    
    // Test that token is not initially revoked
    let is_revoked_before = token_revocation_service.is_token_revoked(jti).await;
    assert!(is_revoked_before.is_ok(), "Should check token revocation status");
    assert!(!is_revoked_before.unwrap(), "Token should not be revoked initially");
    
    // Revoke the token
    let revoke_result = token_revocation_service.revoke_token(
        jti,
        user_id,
        TokenType::Access,
        expires_at,
        Some("Individual test revocation"),
    ).await;
    assert!(revoke_result.is_ok(), "Should revoke token successfully");
    
    // Verify token is now revoked
    let is_revoked_after = token_revocation_service.is_token_revoked(jti).await;
    assert!(is_revoked_after.is_ok(), "Should check token revocation status after revocation");
    assert!(is_revoked_after.unwrap(), "Token should be revoked after revocation");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_cleanup_expired_tokens() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    let token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    let user_id = create_test_db_user(&fixture).await;
    
    // Create an expired token
    let expired_jti = "expired_test_token";
    let expired_time = Utc::now() - Duration::hours(1); // Already expired
    
    let revoke_expired_result = token_revocation_service.revoke_token(
        expired_jti,
        user_id,
        TokenType::Access,
        expired_time,
        Some("Test expired token"),
    ).await;
    assert!(revoke_expired_result.is_ok(), "Should revoke expired token successfully");
    
    // Create a non-expired token
    let valid_jti = "valid_test_token";
    let valid_time = Utc::now() + Duration::hours(1); // Still valid
    
    let revoke_valid_result = token_revocation_service.revoke_token(
        valid_jti,
        user_id,
        TokenType::Refresh,
        valid_time,
        Some("Test valid token"),
    ).await;
    assert!(revoke_valid_result.is_ok(), "Should revoke valid token successfully");
    
    // Cleanup expired tokens
    let cleanup_result = token_revocation_service.cleanup_expired_tokens().await;
    assert!(cleanup_result.is_ok(), "Should cleanup expired tokens successfully");
    assert_eq!(cleanup_result.unwrap(), 1, "Should have cleaned up 1 expired token");
    
    // Verify expired token is gone but valid token remains
    let expired_still_revoked = token_revocation_service.is_token_revoked(expired_jti).await;
    assert!(expired_still_revoked.is_ok(), "Should check expired token status");
    assert!(!expired_still_revoked.unwrap(), "Expired token should be cleaned up (no longer in revoked_tokens table)");
    
    let valid_still_revoked = token_revocation_service.is_token_revoked(valid_jti).await;
    assert!(valid_still_revoked.is_ok(), "Should check valid token status");
    assert!(valid_still_revoked.unwrap(), "Valid token should still be revoked (not cleaned up)");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_token_revocation_with_different_token_types() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    let token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    let user_id = create_test_db_user(&fixture).await;
    let expires_at = Utc::now() + Duration::hours(1);
    
    // Test access token revocation
    let access_jti = "access_token_test";
    let access_result = token_revocation_service.revoke_token(
        access_jti,
        user_id,
        TokenType::Access,
        expires_at,
        Some("Access token revocation test"),
    ).await;
    assert!(access_result.is_ok(), "Should revoke access token successfully");
    
    // Test refresh token revocation
    let refresh_jti = "refresh_token_test";
    let refresh_result = token_revocation_service.revoke_token(
        refresh_jti,
        user_id,
        TokenType::Refresh,
        expires_at,
        Some("Refresh token revocation test"),
    ).await;
    assert!(refresh_result.is_ok(), "Should revoke refresh token successfully");
    
    // Verify both tokens are revoked
    let access_revoked = token_revocation_service.is_token_revoked(access_jti).await;
    assert!(access_revoked.is_ok() && access_revoked.unwrap(), "Access token should be revoked");
    
    let refresh_revoked = token_revocation_service.is_token_revoked(refresh_jti).await;
    assert!(refresh_revoked.is_ok() && refresh_revoked.unwrap(), "Refresh token should be revoked");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_revoke_all_user_tokens_with_invalid_token_types() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create mock active token service with invalid token type
    use oxidizedoasis_websands::core::auth::active_token::MockActiveTokenServiceTrait;
    use mockall::predicate;
    
    let mut mock_active_service = MockActiveTokenServiceTrait::new();
    
    // Create a token with invalid token type
    let invalid_token = ActiveToken {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        jti: "invalid_type_token".to_string(),
        token_type: "invalid_type".to_string(), // Invalid token type
        expires_at: Utc::now() + Duration::hours(1),
        created_at: Utc::now(),
        device_info: None,
    };
    
    mock_active_service
        .expect_get_user_tokens()
        .with(predicate::always())
        .returning(move |_| Ok(vec![invalid_token.clone()]));
    
    mock_active_service
        .expect_remove_all_user_tokens()
        .with(predicate::always())
        .returning(|_| Ok(0));
    
    let active_service_arc = Arc::new(mock_active_service);
    let mut token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    token_revocation_service.set_active_token_service(active_service_arc);
    
    let user_id = Uuid::new_v4();
    
    // Test that invalid token types are skipped gracefully
    let revoke_result = token_revocation_service.revoke_all_user_tokens(user_id, Some("Test invalid token types")).await;
    assert!(revoke_result.is_ok(), "Should handle invalid token types gracefully");
    assert_eq!(revoke_result.unwrap(), 0, "Should skip invalid token types and return 0");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_revoke_all_user_tokens_with_remove_active_tokens_error() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Create mock active token service that fails to remove active tokens
    use oxidizedoasis_websands::core::auth::active_token::MockActiveTokenServiceTrait;
    use mockall::predicate;
    
    let user_id = create_test_db_user(&fixture).await;
    let valid_token = ActiveToken {
        id: Uuid::new_v4(),
        user_id,
        jti: "valid_token_remove_error".to_string(),
        token_type: "access".to_string(),
        expires_at: Utc::now() + Duration::hours(1),
        created_at: Utc::now(),
        device_info: None,
    };
    
    let mut mock_active_service = MockActiveTokenServiceTrait::new();
    
    mock_active_service
        .expect_get_user_tokens()
        .with(predicate::always())
        .returning(move |_| Ok(vec![valid_token.clone()]));
    
    // Make remove_all_user_tokens fail
    mock_active_service
        .expect_remove_all_user_tokens()
        .with(predicate::always())
        .returning(|_| Err(sqlx::Error::RowNotFound));
    
    let active_service_arc = Arc::new(mock_active_service);
    let mut token_revocation_service = TokenRevocationService::new(fixture.db_pool.clone());
    token_revocation_service.set_active_token_service(active_service_arc);
    
    // Test that remove_all_user_tokens error is handled gracefully
    let revoke_result = token_revocation_service.revoke_all_user_tokens(user_id, Some("Test remove error")).await;
    assert!(revoke_result.is_ok(), "Should handle remove_all_user_tokens error gracefully");
    assert_eq!(revoke_result.unwrap(), 1, "Should still revoke tokens even if remove fails");
    
    fixture.cleanup().await;
}