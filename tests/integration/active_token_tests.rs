//! Integration tests for ActiveTokenService
//! Tests the active token management functionality with real database connections

use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use oxidizedoasis_websands::core::auth::{
    active_token::{ActiveTokenService, ActiveTokenServiceTrait},
    jwt::TokenType,
};
use oxidizedoasis_websands::core::user::User;
use test_common::UnifiedTestFixture;

/// Helper function to create a test user in the database
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

#[cfg(test)]
mod record_token_tests {
    use super::*;

    #[tokio::test]
    async fn test_record_access_token_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-access@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "test-access-jti-123";
        let expires_at = Utc::now() + Duration::hours(1);
        let device_info = Some(json!({
            "user_agent": "Mozilla/5.0",
            "ip_address": "127.0.0.1"
        }));

        let result = service.record_token(
            user_id,
            jti,
            TokenType::Access,
            expires_at,
            device_info.clone(),
        ).await;

        assert!(result.is_ok());

        // Verify the token was recorded
        let recorded_token = service.get_active_token(jti).await.unwrap();
        assert_eq!(recorded_token.user_id, user_id);
        assert_eq!(recorded_token.jti, jti);
        assert_eq!(recorded_token.token_type, "access");
        assert_eq!(recorded_token.device_info, device_info);
    }

    #[tokio::test]
    async fn test_record_refresh_token_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-refresh@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "test-refresh-jti-456";
        let expires_at = Utc::now() + Duration::days(30);

        let result = service.record_token(
            user_id,
            jti,
            TokenType::Refresh,
            expires_at,
            None,
        ).await;

        assert!(result.is_ok());

        // Verify the token was recorded with correct type
        let recorded_token = service.get_active_token(jti).await.unwrap();
        assert_eq!(recorded_token.user_id, user_id);
        assert_eq!(recorded_token.jti, jti);
        assert_eq!(recorded_token.token_type, "refresh");
        assert!(recorded_token.device_info.is_none());
    }

    #[tokio::test]
    async fn test_record_duplicate_jti_error() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-duplicate@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "duplicate-jti-789";
        let expires_at = Utc::now() + Duration::hours(1);

        // Record the first token
        let result1 = service.record_token(
            user_id,
            jti,
            TokenType::Access,
            expires_at,
            None,
        ).await;
        assert!(result1.is_ok());

        // Attempt to record the same JTI again
        let result2 = service.record_token(
            user_id,
            jti,
            TokenType::Access,
            expires_at,
            None,
        ).await;
        assert!(result2.is_err());
    }

    #[tokio::test]
    async fn test_record_token_with_complex_device_info() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-complex@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "complex-device-jti";
        let expires_at = Utc::now() + Duration::hours(1);
        let device_info = Some(json!({
            "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
            "ip_address": "192.168.1.100",
            "location": {
                "country": "US",
                "city": "New York"
            },
            "device": {
                "type": "desktop",
                "os": "Windows 10"
            }
        }));

        let result = service.record_token(
            user_id,
            jti,
            TokenType::Access,
            expires_at,
            device_info.clone(),
        ).await;

        assert!(result.is_ok());

        // Verify complex device info is preserved
        let recorded_token = service.get_active_token(jti).await.unwrap();
        assert_eq!(recorded_token.device_info, device_info);
    }
}

#[cfg(test)]
mod remove_token_tests {
    use super::*;

    #[tokio::test]
    async fn test_remove_existing_token_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-remove@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "remove-test-jti";
        let expires_at = Utc::now() + Duration::hours(1);

        // First record a token
        service.record_token(user_id, jti, TokenType::Access, expires_at, None).await.unwrap();
        
        // Verify it exists
        assert!(service.get_active_token(jti).await.is_ok());

        // Remove the token
        let removed = service.remove_token(jti).await.unwrap();
        assert!(removed);

        // Verify it's gone
        assert!(service.get_active_token(jti).await.is_err());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_token() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        let nonexistent_jti = "does-not-exist-jti";
        let removed = service.remove_token(nonexistent_jti).await.unwrap();
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_remove_token_twice() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-twice@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "remove-twice-jti";
        let expires_at = Utc::now() + Duration::hours(1);

        // Record and remove token
        service.record_token(user_id, jti, TokenType::Access, expires_at, None).await.unwrap();
        let removed_first = service.remove_token(jti).await.unwrap();
        assert!(removed_first);

        // Try to remove again
        let removed_second = service.remove_token(jti).await.unwrap();
        assert!(!removed_second);
    }
}

#[cfg(test)]
mod get_active_token_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_existing_active_token() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-get@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "get-token-test";
        let expires_at = Utc::now() + Duration::hours(1);
        let device_info = Some(json!({"test": "data"}));

        // Record a token
        service.record_token(user_id, jti, TokenType::Refresh, expires_at, device_info.clone()).await.unwrap();

        // Get the token
        let retrieved_token = service.get_active_token(jti).await.unwrap();
        
        assert_eq!(retrieved_token.user_id, user_id);
        assert_eq!(retrieved_token.jti, jti);
        assert_eq!(retrieved_token.token_type, "refresh");
        assert_eq!(retrieved_token.device_info, device_info);
        assert!(retrieved_token.expires_at > Utc::now());
    }

    #[tokio::test]
    async fn test_get_nonexistent_active_token() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        let nonexistent_jti = "does-not-exist";
        let result = service.get_active_token(nonexistent_jti).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_removed_token_fails() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-removed@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "get-removed-token";
        let expires_at = Utc::now() + Duration::hours(1);

        // Record and then remove token
        service.record_token(user_id, jti, TokenType::Access, expires_at, None).await.unwrap();
        service.remove_token(jti).await.unwrap();

        // Try to get the removed token
        let result = service.get_active_token(jti).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod get_user_tokens_tests {
    use super::*;

    #[tokio::test]
    async fn test_get_user_tokens_multiple() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-multiple@example.com", "user", true).await;
        let user_id = user.id;
        let expires_at = Utc::now() + Duration::hours(1);

        // Record multiple tokens for the same user
        service.record_token(user_id, "user-token-1", TokenType::Access, expires_at, None).await.unwrap();
        service.record_token(user_id, "user-token-2", TokenType::Refresh, expires_at, None).await.unwrap();
        service.record_token(user_id, "user-token-3", TokenType::Access, expires_at, None).await.unwrap();

        // Get all user tokens
        let tokens = service.get_user_tokens(user_id).await.unwrap();
        
        assert_eq!(tokens.len(), 3);
        let jtis: Vec<&str> = tokens.iter().map(|t| t.jti.as_str()).collect();
        assert!(jtis.contains(&"user-token-1"));
        assert!(jtis.contains(&"user-token-2"));
        assert!(jtis.contains(&"user-token-3"));
    }

    #[tokio::test]
    async fn test_get_user_tokens_empty() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        let user_id = Uuid::new_v4();
        let tokens = service.get_user_tokens(user_id).await.unwrap();
        assert!(tokens.is_empty());
    }

    #[tokio::test]
    async fn test_get_user_tokens_different_users() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create real users in the database
        let user1 = create_test_db_user(&fixture.db_pool, "test-user1@example.com", "user", true).await;
        let user2 = create_test_db_user(&fixture.db_pool, "test-user2@example.com", "user", true).await;
        let user1_id = user1.id;
        let user2_id = user2.id;
        let expires_at = Utc::now() + Duration::hours(1);

        // Record tokens for different users
        service.record_token(user1_id, "user1-token", TokenType::Access, expires_at, None).await.unwrap();
        service.record_token(user2_id, "user2-token", TokenType::Access, expires_at, None).await.unwrap();

        // Get tokens for user1
        let user1_tokens = service.get_user_tokens(user1_id).await.unwrap();
        assert_eq!(user1_tokens.len(), 1);
        assert_eq!(user1_tokens[0].jti, "user1-token");

        // Get tokens for user2
        let user2_tokens = service.get_user_tokens(user2_id).await.unwrap();
        assert_eq!(user2_tokens.len(), 1);
        assert_eq!(user2_tokens[0].jti, "user2-token");
    }

    #[tokio::test]
    async fn test_get_user_tokens_after_partial_removal() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-partial@example.com", "user", true).await;
        let user_id = user.id;
        let expires_at = Utc::now() + Duration::hours(1);

        // Record multiple tokens
        service.record_token(user_id, "keep-token", TokenType::Access, expires_at, None).await.unwrap();
        service.record_token(user_id, "remove-token", TokenType::Refresh, expires_at, None).await.unwrap();

        // Remove one token
        service.remove_token("remove-token").await.unwrap();

        // Get remaining tokens
        let tokens = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].jti, "keep-token");
    }
}

#[cfg(test)]
mod remove_all_user_tokens_tests {
    use super::*;

    #[tokio::test]
    async fn test_remove_all_user_tokens_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-removeall@example.com", "user", true).await;
        let user_id = user.id;
        let expires_at = Utc::now() + Duration::hours(1);

        // Record multiple tokens for the user
        service.record_token(user_id, "remove-all-1", TokenType::Access, expires_at, None).await.unwrap();
        service.record_token(user_id, "remove-all-2", TokenType::Refresh, expires_at, None).await.unwrap();
        service.record_token(user_id, "remove-all-3", TokenType::Access, expires_at, None).await.unwrap();

        // Verify tokens exist
        let tokens_before = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(tokens_before.len(), 3);

        // Remove all user tokens
        let removed_count = service.remove_all_user_tokens(user_id).await.unwrap();
        assert_eq!(removed_count, 3);

        // Verify all tokens are gone
        let tokens_after = service.get_user_tokens(user_id).await.unwrap();
        assert!(tokens_after.is_empty());
    }

    #[tokio::test]
    async fn test_remove_all_user_tokens_no_tokens() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        let user_id = Uuid::new_v4();
        let removed_count = service.remove_all_user_tokens(user_id).await.unwrap();
        assert_eq!(removed_count, 0);
    }

    #[tokio::test]
    async fn test_remove_all_user_tokens_preserves_other_users() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create real users in the database
        let user1 = create_test_db_user(&fixture.db_pool, "test-preserve1@example.com", "user", true).await;
        let user2 = create_test_db_user(&fixture.db_pool, "test-preserve2@example.com", "user", true).await;
        let user1_id = user1.id;
        let user2_id = user2.id;
        let expires_at = Utc::now() + Duration::hours(1);

        // Record tokens for both users
        service.record_token(user1_id, "user1-token", TokenType::Access, expires_at, None).await.unwrap();
        service.record_token(user2_id, "user2-token", TokenType::Access, expires_at, None).await.unwrap();

        // Remove all tokens for user1
        let removed_count = service.remove_all_user_tokens(user1_id).await.unwrap();
        assert_eq!(removed_count, 1);

        // Verify user1 tokens are gone but user2 tokens remain
        let user1_tokens = service.get_user_tokens(user1_id).await.unwrap();
        assert!(user1_tokens.is_empty());

        let user2_tokens = service.get_user_tokens(user2_id).await.unwrap();
        assert_eq!(user2_tokens.len(), 1);
        assert_eq!(user2_tokens[0].jti, "user2-token");
    }
}

#[cfg(test)]
mod cleanup_expired_tokens_tests {
    use super::*;

    #[tokio::test]
    async fn test_cleanup_expired_tokens_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-cleanup@example.com", "user", true).await;
        let user_id = user.id;
        
        // Record expired tokens
        let past_time = Utc::now() - Duration::hours(1);
        service.record_token(user_id, "expired-1", TokenType::Access, past_time, None).await.unwrap();
        service.record_token(user_id, "expired-2", TokenType::Refresh, past_time, None).await.unwrap();
        
        // Record valid token
        let future_time = Utc::now() + Duration::hours(1);
        service.record_token(user_id, "valid-token", TokenType::Access, future_time, None).await.unwrap();

        // Clean up expired tokens
        let cleaned_count = service.cleanup_expired_tokens().await.unwrap();
        assert_eq!(cleaned_count, 2);

        // Verify expired tokens are gone
        assert!(service.get_active_token("expired-1").await.is_err());
        assert!(service.get_active_token("expired-2").await.is_err());
        
        // Verify valid token remains
        assert!(service.get_active_token("valid-token").await.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens_none_expired() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-cleanup-none@example.com", "user", true).await;
        let user_id = user.id;
        let future_time = Utc::now() + Duration::hours(1);
        
        // Record only valid tokens
        service.record_token(user_id, "valid-1", TokenType::Access, future_time, None).await.unwrap();
        service.record_token(user_id, "valid-2", TokenType::Refresh, future_time, None).await.unwrap();

        // Clean up expired tokens
        let cleaned_count = service.cleanup_expired_tokens().await.unwrap();
        assert_eq!(cleaned_count, 0);

        // Verify all tokens still exist
        assert!(service.get_active_token("valid-1").await.is_ok());
        assert!(service.get_active_token("valid-2").await.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens_boundary_case() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-boundary@example.com", "user", true).await;
        let user_id = user.id;
        
        // Record token expiring right now (boundary case)
        let now = Utc::now();
        service.record_token(user_id, "boundary-token", TokenType::Access, now, None).await.unwrap();
        
        // Small delay to ensure the token is now expired
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Clean up expired tokens
        let cleaned_count = service.cleanup_expired_tokens().await.unwrap();
        assert!(cleaned_count >= 1); // Should clean up the boundary token
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens_empty_table() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Clean up on empty table
        let cleaned_count = service.cleanup_expired_tokens().await.unwrap();
        assert_eq!(cleaned_count, 0);
    }
}

#[cfg(test)]
mod integration_workflow_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_token_lifecycle() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-lifecycle@example.com", "user", true).await;
        let user_id = user.id;
        let access_jti = "lifecycle-access";
        let refresh_jti = "lifecycle-refresh";
        let expires_at = Utc::now() + Duration::hours(1);

        // 1. Record tokens
        service.record_token(user_id, access_jti, TokenType::Access, expires_at, None).await.unwrap();
        service.record_token(user_id, refresh_jti, TokenType::Refresh, expires_at, None).await.unwrap();

        // 2. Verify tokens exist
        let user_tokens = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(user_tokens.len(), 2);

        // 3. Remove access token (simulating logout)
        let removed = service.remove_token(access_jti).await.unwrap();
        assert!(removed);

        // 4. Verify only refresh token remains
        let remaining_tokens = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(remaining_tokens.len(), 1);
        assert_eq!(remaining_tokens[0].jti, refresh_jti);

        // 5. Remove all user tokens (simulating full logout)
        let removed_count = service.remove_all_user_tokens(user_id).await.unwrap();
        assert_eq!(removed_count, 1);

        // 6. Verify no tokens remain
        let final_tokens = service.get_user_tokens(user_id).await.unwrap();
        assert!(final_tokens.is_empty());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-concurrent@example.com", "user", true).await;
        let user_id = user.id;
        let expires_at = Utc::now() + Duration::hours(1);

        // Simulate concurrent token operations
        let mut handles = vec![];

        for i in 0..5 {
            let service_clone = ActiveTokenService::new(fixture.db_pool.clone());
            let jti = format!("concurrent-token-{}", i);
            
            let handle = tokio::spawn(async move {
                service_clone.record_token(user_id, &jti, TokenType::Access, expires_at, None).await
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }

        // Verify all tokens were recorded
        let user_tokens = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(user_tokens.len(), 5);
    }

    #[tokio::test]
    async fn test_expired_and_active_tokens_mixed() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-mixed@example.com", "user", true).await;
        let user_id = user.id;
        
        // Record mix of expired and active tokens
        let past_time = Utc::now() - Duration::hours(1);
        let future_time = Utc::now() + Duration::hours(1);
        
        service.record_token(user_id, "expired-token-1", TokenType::Access, past_time, None).await.unwrap();
        service.record_token(user_id, "active-token-1", TokenType::Access, future_time, None).await.unwrap();
        service.record_token(user_id, "expired-token-2", TokenType::Refresh, past_time, None).await.unwrap();
        service.record_token(user_id, "active-token-2", TokenType::Refresh, future_time, None).await.unwrap();

        // Verify all tokens exist before cleanup
        let all_tokens = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(all_tokens.len(), 4);

        // Clean up expired tokens
        let cleaned_count = service.cleanup_expired_tokens().await.unwrap();
        assert_eq!(cleaned_count, 2);

        // Verify only active tokens remain
        let remaining_tokens = service.get_user_tokens(user_id).await.unwrap();
        assert_eq!(remaining_tokens.len(), 2);
        
        let remaining_jtis: Vec<&str> = remaining_tokens.iter().map(|t| t.jti.as_str()).collect();
        assert!(remaining_jtis.contains(&"active-token-1"));
        assert!(remaining_jtis.contains(&"active-token-2"));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_service_with_invalid_database_connection() {
        // Create service with invalid database URL to test error handling
        let invalid_pool = PgPool::connect("postgresql://invalid:invalid@localhost:5432/invalid")
            .await
            .expect_err("Should fail to connect to invalid database");
        
        // The connection should fail, so we can't actually test with an invalid pool
        // Instead, we'll test with a valid pool and simulate errors through other means
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Test getting a non-existent token (this should return a database error)
        let result = service.get_active_token("absolutely-does-not-exist").await;
        assert!(result.is_err());
        
        // The error should be a SqlxError::RowNotFound
        match result.unwrap_err() {
            sqlx::Error::RowNotFound => {
                // This is expected
            }
            other => panic!("Expected RowNotFound error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_database_constraint_violations() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let service = ActiveTokenService::new(fixture.db_pool.clone());
        
        // Create a real user in the database
        let user = create_test_db_user(&fixture.db_pool, "test-constraint@example.com", "user", true).await;
        let user_id = user.id;
        let jti = "constraint-test-jti";
        let expires_at = Utc::now() + Duration::hours(1);

        // Record initial token
        let result1 = service.record_token(user_id, jti, TokenType::Access, expires_at, None).await;
        assert!(result1.is_ok());

        // Try to record with same JTI (should violate unique constraint)
        let result2 = service.record_token(user_id, jti, TokenType::Access, expires_at, None).await;
        assert!(result2.is_err());
    }
}