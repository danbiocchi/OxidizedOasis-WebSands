//! Integration tests for UserRepository
//! 
//! This test suite covers all UserRepositoryTrait methods with real database operations,
//! ensuring proper integration with PostgreSQL and comprehensive coverage of:
//! - User creation and CRUD operations
//! - Email verification workflows
//! - Password reset token management
//! - Role management and updates
//! - Error handling and edge cases

use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;
use sqlx::PgPool;
use tokio;

// Import test infrastructure
use test_common::UnifiedTestFixture;

// Import the modules under test
use oxidizedoasis_websands::core::user::repository::{UserRepository, UserRepositoryTrait};
use oxidizedoasis_websands::core::user::model::{User, NewUser, PasswordResetToken};
use oxidizedoasis_websands::common::validation::UserInput;

/// Helper function to create a test UserInput with unique identifiers
fn create_test_user_input(suffix: &str) -> UserInput {
    UserInput {
        username: format!("testuser_{}", suffix),
        email: Some(format!("test_{}@example.com", suffix)),
        password: Some("TestPassword123!".to_string()),
    }
}

/// Helper function to create a test NewUser with unique identifiers
fn create_test_new_user(suffix: &str) -> NewUser {
    NewUser {
        username: format!("newuser_{}", suffix),
        email: Some(format!("new_{}@example.com", suffix)),
        password_hash: format!("hashed_password_{}", suffix),
        is_email_verified: false,
        role: "user".to_string(),
        verification_token: Some(format!("verification_token_{}", suffix)),
        verification_token_expires_at: Some(Utc::now() + Duration::hours(24)),
    }
}

/// Helper function to clean up test users by username pattern
async fn cleanup_test_users(pool: &PgPool, username_pattern: &str) {
    let _ = sqlx::query("DELETE FROM password_reset_tokens WHERE user_id IN (SELECT id FROM users WHERE username LIKE $1)")
        .bind(format!("{}%", username_pattern))
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE username LIKE $1")
        .bind(format!("{}%", username_pattern))
        .execute(pool)
        .await;
}

#[cfg(test)]
mod user_repository_integration_tests {
    use super::*;

    /// Test creating a user with complete details using create_user_with_details
    #[tokio::test]
    async fn test_create_user_with_details_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        // Clean up any existing test data
        cleanup_test_users(&fixture.db_pool, "testuser_create_details").await;

        let user_input = create_test_user_input("create_details");
        let password_hash = "hashed_password_123";
        let verification_token = "verification_token_123";

        let result = repository
            .create_user_with_details(&user_input, password_hash.to_string(), verification_token.to_string())
            .await;

        assert!(result.is_ok(), "Failed to create user with details: {:?}", result.err());
        
        let user = result.unwrap();
        assert_eq!(user.username, "testuser_create_details");
        assert_eq!(user.email, Some("test_create_details@example.com".to_string()));
        assert_eq!(user.password_hash, "hashed_password_123");
        assert_eq!(user.role, "user");
        assert!(!user.is_email_verified);
        assert!(user.is_active);
        assert!(user.verification_token.is_some());
    }

    /// Test creating a user with NewUser struct
    #[tokio::test]
    async fn test_create_user_with_new_user_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "newuser_create").await;

        let new_user = create_test_new_user("create");

        let result = repository.create_user(new_user).await;

        assert!(result.is_ok(), "Failed to create user: {:?}", result.err());
        
        let user = result.unwrap();
        assert_eq!(user.username, "newuser_create");
        assert_eq!(user.email, Some("new_create@example.com".to_string()));
        assert_eq!(user.role, "user");
        assert!(!user.is_email_verified);
        assert!(user.is_active);
    }

    /// Test finding a user by ID
    #[tokio::test]
    async fn test_find_by_id_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_find_id").await;

        // First create a user
        let user_input = create_test_user_input("find_id");
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Then find by ID
        let result = repository.find_by_id(created_user.id).await;

        assert!(result.is_ok(), "Failed to find user by ID: {:?}", result.err());
        
        let found_user = result.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.id, created_user.id);
        assert_eq!(found_user.username, "testuser_find_id");
    }

    /// Test finding a user by ID that doesn't exist
    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        let non_existent_id = Uuid::new_v4();
        let result = repository.find_by_id(non_existent_id).await;

        assert!(result.is_ok(), "Query should succeed even if user not found");
        assert!(result.unwrap().is_none(), "Should return None for non-existent user");
    }

    /// Test finding a user by username
    #[tokio::test]
    async fn test_find_by_username_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_find_username").await;

        // Create a user
        let user_input = create_test_user_input("find_username");
        let _created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Find by username
        let result = repository.find_by_username("testuser_find_username").await;

        assert!(result.is_ok(), "Failed to find user by username: {:?}", result.err());
        
        let found_user = result.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.username, "testuser_find_username");
    }

    /// Test finding a user by email using find_user_by_email
    #[tokio::test]
    async fn test_find_user_by_email_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_find_email").await;

        // Create a user
        let user_input = create_test_user_input("find_email");
        let _created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Find by email using correct method name
        let result = repository.find_user_by_email("test_find_email@example.com").await;

        assert!(result.is_ok(), "Failed to find user by email: {:?}", result.err());
        
        let found_user = result.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(user_input.email, found_user.email);
    }

    /// Test finding a user by verification token
    #[tokio::test]
    async fn test_find_by_verification_token_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_find_token").await;

        // Create a user with verification token
        let user_input = create_test_user_input("find_token");
        let verification_token = "unique_verification_token_123";
        let _created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), verification_token.to_string())
            .await
            .expect("Failed to create test user");

        // Find by verification token
        let result = repository.find_by_verification_token(verification_token).await;

        assert!(result.is_ok(), "Failed to find user by verification token: {:?}", result.err());
        
        let found_user = result.unwrap();
        assert!(found_user.is_some());
        let found_user = found_user.unwrap();
        assert_eq!(found_user.username, "testuser_find_token");
        assert_eq!(found_user.verification_token, Some(verification_token.to_string()));
    }

    /// Test verifying a user's email using verify_email
    #[tokio::test]
    async fn test_verify_email_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_verify").await;

        // Create an unverified user
        let user_input = create_test_user_input("verify");
        let verification_token = "verify_token_123";
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), verification_token.to_string())
            .await
            .expect("Failed to create test user");

        assert!(!created_user.is_email_verified, "User should initially be unverified");

        // Verify the user using verify_email method
        let result = repository.verify_email(verification_token).await;

        assert!(result.is_ok(), "Failed to verify email: {:?}", result.err());
        
        let user_id_option = result.unwrap();
        assert!(user_id_option.is_some(), "Should return user ID after verification");
        let user_id = user_id_option.unwrap();
        assert_eq!(user_id, created_user.id);

        // Verify the user is now marked as verified
        let updated_user = repository.find_by_id(created_user.id).await.unwrap().unwrap();
        assert!(updated_user.is_email_verified, "User should be verified after verify_email call");
        assert!(updated_user.verification_token.is_none(), "Verification token should be cleared");
    }

    /// Test updating email and setting user as unverified
    #[tokio::test]
    async fn test_update_email_and_set_unverified_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_email_update").await;

        // Create a verified user
        let user_input = create_test_user_input("email_update");
        let mut created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Verify the user first using verify_email
        repository.verify_email("token123").await.expect("Failed to verify user");
        created_user = repository.find_by_id(created_user.id).await.unwrap().unwrap();
        assert!(created_user.is_email_verified, "User should be verified");

        // Update email and set as unverified
        let new_email = "updated_email@example.com";
        let new_verification_token = "new_verification_token_123";
        let result = repository
            .update_email_and_set_unverified(created_user.id, new_email, new_verification_token)
            .await;

        assert!(result.is_ok(), "Failed to update email: {:?}", result.err());
        
        let updated_user = result.unwrap();
        assert_eq!(updated_user.email, Some(new_email.to_string()));
        assert!(!updated_user.is_email_verified, "User should be unverified after email update");
        assert_eq!(updated_user.verification_token, Some(new_verification_token.to_string()));
    }

    /// Test updating user with UserInput
    #[tokio::test]
    async fn test_update_user_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_update").await;

        // Create a user
        let user_input = create_test_user_input("update");
        let created_user = repository
            .create_user_with_details(&user_input, "original_hash".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Update user
        let update_input = UserInput {
            username: "updated_username".to_string(),
            email: Some("updated@example.com".to_string()),
            password: None, // Not updating password
        };
        let result = repository.update(created_user.id, &update_input, None).await;

        assert!(result.is_ok(), "Failed to update user: {:?}", result.err());
        
        let updated_user = result.unwrap();
        assert_eq!(updated_user.username, "updated_username");
        assert_eq!(updated_user.email, Some("updated@example.com".to_string()));
        assert_eq!(updated_user.password_hash, "original_hash"); // Should remain unchanged
    }

    /// Test updating user with new password
    #[tokio::test]
    async fn test_update_user_with_password_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_update_pwd").await;

        // Create a user
        let user_input = create_test_user_input("update_pwd");
        let created_user = repository
            .create_user_with_details(&user_input, "original_hash".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Update user with new password
        let update_input = UserInput {
            username: created_user.username.clone(),
            email: created_user.email.clone(),
            password: Some("NewPassword123!".to_string()),
        };
        let new_password_hash = "new_hashed_password";
        let result = repository.update(created_user.id, &update_input, Some(new_password_hash.to_string())).await;

        assert!(result.is_ok(), "Failed to update user with password: {:?}", result.err());
        
        let updated_user = result.unwrap();
        assert_eq!(updated_user.password_hash, "new_hashed_password");
    }

    /// Test updating user role
    #[tokio::test]
    async fn test_update_role_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_role_update").await;

        // Create a user with default "user" role
        let user_input = create_test_user_input("role_update");
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        assert_eq!(created_user.role, "user");

        // Update role to admin
        let result = repository.update_role(created_user.id, "admin").await;

        assert!(result.is_ok(), "Failed to update user role: {:?}", result.err());
        
        let updated_user = result.unwrap();
        assert!(updated_user.is_some(), "Should return updated user");
        let updated_user = updated_user.unwrap();
        assert_eq!(updated_user.role, "admin");
    }

    /// Test updating role for non-existent user
    #[tokio::test]
    async fn test_update_role_user_not_found() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        let non_existent_id = Uuid::new_v4();
        let result = repository.update_role(non_existent_id, "admin").await;

        assert!(result.is_ok(), "Query should succeed even if user not found");
        assert!(result.unwrap().is_none(), "Should return None for non-existent user");
    }

    /// Test deleting a user
    #[tokio::test]
    async fn test_delete_user_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_delete").await;

        // Create a user
        let user_input = create_test_user_input("delete");
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Verify user exists
        let found_user = repository.find_by_id(created_user.id).await.expect("Failed to find user");
        assert!(found_user.is_some(), "User should exist before deletion");

        // Delete user
        let result = repository.delete(created_user.id).await;

        assert!(result.is_ok(), "Failed to delete user: {:?}", result.err());
        
        let was_deleted = result.unwrap();
        assert!(was_deleted, "Should return true when user was deleted");

        // Verify user no longer exists
        let found_user = repository.find_by_id(created_user.id).await.expect("Failed to query for user");
        assert!(found_user.is_none(), "User should not exist after deletion");
    }

    /// Test deleting non-existent user
    #[tokio::test]
    async fn test_delete_user_not_found() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        let non_existent_id = Uuid::new_v4();
        let result = repository.delete(non_existent_id).await;

        assert!(result.is_ok(), "Delete query should succeed even if user not found");
        assert!(!result.unwrap(), "Should return false when no user was deleted");
    }

    /// Test creating password reset token
    #[tokio::test]
    async fn test_create_password_reset_token_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_reset_token").await;

        // Create a user
        let user_input = create_test_user_input("reset_token");
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Create password reset token (method signature only takes user_id)
        let result = repository.create_password_reset_token(created_user.id).await;

        assert!(result.is_ok(), "Failed to create password reset token: {:?}", result.err());
        
        let token = result.unwrap();
        assert_eq!(token.user_id, created_user.id);
        assert!(!token.token.is_empty());
        assert!(!token.is_used);
        assert!(token.expires_at > Utc::now());
    }

    /// Test verifying password reset token
    #[tokio::test]
    async fn test_verify_reset_token_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_verify_reset").await;

        // Create a user and reset token
        let user_input = create_test_user_input("verify_reset");
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        let created_token = repository
            .create_password_reset_token(created_user.id)
            .await
            .expect("Failed to create reset token");

        // Verify password reset token
        let result = repository.verify_reset_token(&created_token.token).await;

        assert!(result.is_ok(), "Failed to verify password reset token: {:?}", result.err());
        
        let found_token = result.unwrap();
        assert!(found_token.is_some(), "Should find the reset token");
        let found_token = found_token.unwrap();
        assert_eq!(found_token.token, created_token.token);
        assert_eq!(found_token.user_id, created_user.id);
        assert!(!found_token.is_used);
    }

    /// Test marking password reset token as used
    #[tokio::test]
    async fn test_mark_reset_token_used_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_mark_reset").await;

        // Create a user and reset token
        let user_input = create_test_user_input("mark_reset");
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        let created_token = repository
            .create_password_reset_token(created_user.id)
            .await
            .expect("Failed to create reset token");

        // Verify token exists and is not used
        let found_token = repository.verify_reset_token(&created_token.token).await.expect("Failed to find token");
        assert!(found_token.is_some(), "Token should exist before marking as used");

        // Mark password reset token as used
        let result = repository.mark_reset_token_used(&created_token.token).await;

        assert!(result.is_ok(), "Failed to mark password reset token as used: {:?}", result.err());
        
        let was_marked = result.unwrap();
        assert!(was_marked, "Should return true when token was marked as used");

        // Verify token can no longer be verified (should be marked as used)
        let found_token = repository.verify_reset_token(&created_token.token).await.expect("Failed to query for token");
        assert!(found_token.is_none(), "Token should not be verifiable after being marked as used");
    }

    /// Test updating user password using update_password
    #[tokio::test]
    async fn test_update_password_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_update_password").await;

        // Create a user
        let user_input = create_test_user_input("update_password");
        let created_user = repository
            .create_user_with_details(&user_input, "original_hash".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Update password using update_password method
        let new_password_hash = "new_password_hash_123";
        let result = repository.update_password(created_user.id, new_password_hash).await;

        assert!(result.is_ok(), "Failed to update password: {:?}", result.err());

        // Verify the password was updated
        let updated_user = repository.find_by_id(created_user.id).await.unwrap().unwrap();
        assert_eq!(updated_user.password_hash, new_password_hash);
        assert_eq!(updated_user.id, created_user.id);
    }

    /// Test duplicate username constraint
    #[tokio::test]
    async fn test_create_user_duplicate_username_error() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_duplicate").await;

        // Create first user
        let user_input1 = UserInput {
            username: "testuser_duplicate".to_string(),
            email: Some("first@example.com".to_string()),
            password: Some("Password123!".to_string()),
        };
        let _first_user = repository
            .create_user_with_details(&user_input1, "hash1".to_string(), "token1".to_string())
            .await
            .expect("Failed to create first user");

        // Try to create second user with same username but different email
        let user_input2 = UserInput {
            username: "testuser_duplicate".to_string(), // Same username
            email: Some("second@example.com".to_string()), // Different email
            password: Some("Password123!".to_string()),
        };
        let result = repository
            .create_user_with_details(&user_input2, "hash2".to_string(), "token2".to_string())
            .await;

        assert!(result.is_err(), "Should fail when creating user with duplicate username");
    }

    /// Test duplicate email constraint
    #[tokio::test]
    async fn test_create_user_duplicate_email_error() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_email_dup").await;

        // Create first user
        let user_input1 = UserInput {
            username: "testuser_email_dup1".to_string(),
            email: Some("duplicate@example.com".to_string()),
            password: Some("Password123!".to_string()),
        };
        let _first_user = repository
            .create_user_with_details(&user_input1, "hash1".to_string(), "token1".to_string())
            .await
            .expect("Failed to create first user");

        // Try to create second user with different username but same email
        let user_input2 = UserInput {
            username: "testuser_email_dup2".to_string(), // Different username
            email: Some("duplicate@example.com".to_string()), // Same email
            password: Some("Password123!".to_string()),
        };
        let result = repository
            .create_user_with_details(&user_input2, "hash2".to_string(), "token2".to_string())
            .await;

        assert!(result.is_err(), "Should fail when creating user with duplicate email");
    }

    /// Test edge case: user with None email
    #[tokio::test]
    async fn test_create_user_with_none_email() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_no_email").await;

        let user_input = UserInput {
            username: "testuser_no_email".to_string(),
            email: None, // No email
            password: Some("Password123!".to_string()),
        };

        let result = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await;

        assert!(result.is_ok(), "Should be able to create user without email: {:?}", result.err());
        
        let user = result.unwrap();
        assert_eq!(user.username, "testuser_no_email");
        assert!(user.email.is_none(), "Email should be None");
    }

    /// Test find_by_email_and_verified method
    #[tokio::test]
    async fn test_find_by_email_and_verified_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_verified_email").await;

        // Create a user and verify their email
        let user_input = create_test_user_input("verified_email");
        let verification_token = "verify_token_123";
        let created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), verification_token.to_string())
            .await
            .expect("Failed to create test user");

        // Verify the email
        repository.verify_email(verification_token).await.expect("Failed to verify email");

        // Find by email and verified status
        let result = repository.find_by_email_and_verified("test_verified_email@example.com").await;

        assert!(result.is_ok(), "Failed to find verified user by email: {:?}", result.err());
        
        let found_user = result.unwrap();
        assert!(found_user.is_some(), "Should find the verified user");
        let found_user = found_user.unwrap();
        assert_eq!(found_user.id, created_user.id);
        assert!(found_user.is_email_verified, "User should be verified");
    }

    /// Test find_by_email_and_verified with unverified user
    #[tokio::test]
    async fn test_find_by_email_and_verified_unverified_user() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_unverified_email").await;

        // Create a user but don't verify their email
        let user_input = create_test_user_input("unverified_email");
        let _created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), "token123".to_string())
            .await
            .expect("Failed to create test user");

        // Try to find by email and verified status (should not find unverified user)
        let result = repository.find_by_email_and_verified("test_unverified_email@example.com").await;

        assert!(result.is_ok(), "Query should succeed");
        assert!(result.unwrap().is_none(), "Should not find unverified user");
    }

    /// Test check_email_verified method
    #[tokio::test]
    async fn test_check_email_verified_success() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_check_verified").await;

        // Create a user and verify their email
        let user_input = create_test_user_input("check_verified");
        let verification_token = "verify_token_123";
        let _created_user = repository
            .create_user_with_details(&user_input, "hash123".to_string(), verification_token.to_string())
            .await
            .expect("Failed to create test user");

        // Check before verification
        let is_verified_before = repository.check_email_verified("testuser_check_verified").await.unwrap();
        assert!(!is_verified_before, "User should not be verified initially");

        // Verify the email
        repository.verify_email(verification_token).await.expect("Failed to verify email");

        // Check after verification
        let is_verified_after = repository.check_email_verified("testuser_check_verified").await.unwrap();
        assert!(is_verified_after, "User should be verified after verification");
    }

    /// Test find_all method
    #[tokio::test]
    async fn test_find_all_users() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        let repository = UserRepository::new(fixture.db_pool.clone());
        
        cleanup_test_users(&fixture.db_pool, "testuser_find_all").await;

        // Create multiple test users
        let user_input1 = create_test_user_input("find_all_1");
        let user_input2 = create_test_user_input("find_all_2");

        let _user1 = repository
            .create_user_with_details(&user_input1, "hash1".to_string(), "token1".to_string())
            .await
            .expect("Failed to create first user");

        let _user2 = repository
            .create_user_with_details(&user_input2, "hash2".to_string(), "token2".to_string())
            .await
            .expect("Failed to create second user");

        // Find all users
        let result = repository.find_all().await;

        assert!(result.is_ok(), "Failed to find all users: {:?}", result.err());
        
        let users = result.unwrap();
        assert!(users.len() >= 2, "Should find at least the 2 test users we created");
        
        let test_usernames: Vec<&str> = users.iter()
            .filter(|u| u.username.starts_with("testuser_find_all"))
            .map(|u| u.username.as_str())
            .collect();
        assert!(test_usernames.contains(&"testuser_find_all_1"));
        assert!(test_usernames.contains(&"testuser_find_all_2"));
    }
}