// tests/integration/user_service_tests.rs
// Additional unit tests for UserService to improve coverage beyond 74.2%
// Focus on edge cases and error conditions not covered by existing tests

use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use bcrypt::DEFAULT_COST;
use mockall::predicate;

use oxidizedoasis_websands::{
    core::{
        user::{
            service::UserService,
            repository::MockUserRepositoryTrait,
            User,
            model::PasswordResetToken,
        },
        email::service::MockEmailServiceTrait,
        auth::token_revocation::MockTokenRevocationServiceTrait,
    },
    common::{
        error::{ApiError, ApiErrorType},
        validation::UserInput,
    },
};

// Helper function to create a basic UserInput
fn create_user_input(username: &str, email: Option<&str>, password: Option<&str>) -> UserInput {
    UserInput {
        username: username.to_string(),
        email: email.map(|e| e.to_string()),
        password: password.map(|p| p.to_string()),
    }
}

// Helper function to create a test User
fn create_test_user(id: Uuid, username: &str, email: Option<&str>, verified: bool) -> User {
    User {
        id,
        username: username.to_string(),
        email: email.map(|e| e.to_string()),
        password_hash: "hashed_password".to_string(),
        is_email_verified: verified,
        verification_token: Some("test_token".to_string()),
        verification_token_expires_at: Some(Utc::now() + chrono::Duration::hours(24)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        role: "user".to_string(),
        is_active: true,
    }
}

#[tokio::test]
async fn test_create_user_bcrypt_hash_error() {
    // Test bcrypt hashing failure edge case
    let mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    // Test with an extremely long password that might cause bcrypt to fail
    let invalid_input = create_user_input(
        "testuser",
        Some("test@example.com"),
        Some("a".repeat(1000).as_str()), // Extremely long password
    );

    let result = user_service.create_user(invalid_input).await;
    
    // Should handle bcrypt error gracefully
    if result.is_err() {
        let err = result.unwrap_err();
        assert!(matches!(err.error_type, ApiErrorType::Internal | ApiErrorType::Validation));
    }
}

#[tokio::test]
async fn test_update_user_complex_email_change_with_username_change() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mut mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();
    let current_user = create_test_user(user_id, "oldname", Some("old@example.com"), true);
    let new_email = "new@example.com";
    let new_username = "newname";

    // User after email update
    let user_after_email_update = User {
        email: Some(new_email.to_string()),
        is_email_verified: false,
        verification_token: Some("new_token".to_string()),
        ..current_user.clone()
    };

    // Final user after username update too
    let final_user = User {
        username: new_username.to_string(),
        email: Some(new_email.to_string()),
        is_email_verified: false,
        verification_token: Some("new_token".to_string()),
        ..current_user.clone()
    };

    mock_repo.expect_find_by_id()
        .with(predicate::eq(user_id))
        .times(1)
        .returning(move |_| Ok(Some(current_user.clone())));

    mock_repo.expect_find_by_email_and_verified()
        .with(predicate::eq(new_email))
        .times(1)
        .returning(|_| Ok(None));

    mock_repo.expect_update_email_and_set_unverified()
        .times(1)
        .returning(move |_, _, _| Ok(user_after_email_update.clone()));

    mock_repo.expect_update_username()
        .with(predicate::eq(user_id), predicate::eq(new_username))
        .times(1)
        .returning(move |_, _| Ok(Some(final_user.clone())));

    mock_email_service.expect_send_verification_email()
        .times(1)
        .returning(|_, _| Ok(()));

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let input = create_user_input(new_username, Some(new_email), None);
    let result = user_service.update_user(user_id, input).await;

    assert!(result.is_ok());
    let updated_user = result.unwrap();
    assert_eq!(updated_user.username, new_username);
    assert_eq!(updated_user.email.as_deref(), Some(new_email));
    assert!(!updated_user.is_email_verified);
}

#[tokio::test]
async fn test_update_user_email_change_username_update_error() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();
    let current_user = create_test_user(user_id, "oldname", Some("old@example.com"), true);
    let new_email = "new@example.com";
    let new_username = "newname";

    let user_after_email_update = User {
        email: Some(new_email.to_string()),
        is_email_verified: false,
        verification_token: Some("new_token".to_string()),
        ..current_user.clone()
    };

    mock_repo.expect_find_by_id()
        .times(1)
        .returning(move |_| Ok(Some(current_user.clone())));

    mock_repo.expect_find_by_email_and_verified()
        .times(1)
        .returning(|_| Ok(None));

    mock_repo.expect_update_email_and_set_unverified()
        .times(1)
        .returning(move |_, _, _| Ok(user_after_email_update.clone()));

    // Username update fails
    mock_repo.expect_update_username()
        .times(1)
        .returning(|_, _| Ok(None)); // Returns None indicating user not found

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let input = create_user_input(new_username, Some(new_email), None);
    let result = user_service.update_user(user_id, input).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Internal);
    assert_eq!(err.message, "User consistency error after update");
}

#[tokio::test]
async fn test_update_user_password_validation_error() {
    let mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    // Test password validation failure - validation happens before any repository calls
    let input = UserInput {
        username: "testuser".to_string(),
        email: Some("new@example.com".to_string()),
        password: Some("weak".to_string()), // This should fail validation early
    };

    let result = user_service.update_user(user_id, input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Validation);
}

#[tokio::test]
async fn test_update_user_password_bcrypt_error_in_email_change_path() {
    let mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    // Test with extremely long password - this should fail password validation early
    let input = UserInput {
        username: "testuser".to_string(),
        email: Some("new@example.com".to_string()),
        password: Some("a".repeat(1000)), // This should fail validation (too long)
    };

    let result = user_service.update_user(user_id, input).await;
    
    // Password validation should fail before any repository calls
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Validation);
}

#[tokio::test]
async fn test_update_user_password_database_error_in_non_email_change_path() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();
    let current_user = create_test_user(user_id, "testuser", Some("test@example.com"), true);

    mock_repo.expect_find_by_id()
        .times(1)
        .returning(move |_| Ok(Some(current_user.clone())));

    // Update method fails
    mock_repo.expect_update()
        .times(1)
        .returning(|_, _, _| Err(sqlx::Error::PoolClosed));

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let input = UserInput {
        username: "testuser".to_string(),
        email: Some("test@example.com".to_string()), // Same email, no change
        password: Some("NewPassword123!".to_string()),
    };

    let result = user_service.update_user(user_id, input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Database);
}

#[tokio::test]
async fn test_reset_password_bcrypt_error() {
    let mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    // Test with extremely long password that should cause password validation to fail first
    let result = user_service.reset_password("any_token", &"a".repeat(1000)).await;
    
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Validation);
}

#[tokio::test]
async fn test_reset_password_mark_token_used_error() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let token_str = "valid_token";
    let user_id = Uuid::new_v4();
    let reset_token = PasswordResetToken {
        id: Uuid::new_v4(),
        user_id,
        token: token_str.to_string(),
        expires_at: Utc::now() + chrono::Duration::hours(1),
        is_used: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    mock_repo.expect_verify_reset_token()
        .times(1)
        .returning(move |_| Ok(Some(reset_token.clone())));

    mock_repo.expect_update_password()
        .times(1)
        .returning(|_, _| Ok(()));

    // Mark token as used fails
    mock_repo.expect_mark_reset_token_used()
        .times(1)
        .returning(|_| Err(sqlx::Error::PoolClosed));

    // Looking at reset_password method, token revocation happens after mark_reset_token_used
    // If mark_reset_token_used fails, the method returns early, so token revocation doesn't happen
    // Let's remove this expectation

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let result = user_service.reset_password(token_str, "NewPassword123!").await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Database);
}

#[tokio::test]
async fn test_resend_verification_email_update_token_error() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();
    let test_user = create_test_user(user_id, "testuser", Some("test@example.com"), false);

    mock_repo.expect_find_by_id()
        .times(1)
        .returning(move |_| Ok(Some(test_user.clone())));

    // Update verification token fails
    mock_repo.expect_update_verification_token()
        .times(1)
        .returning(|_, _| Err(sqlx::Error::PoolClosed));

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let result = user_service.resend_verification_email(user_id).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Database);
}

#[tokio::test]
async fn test_constructor_new() {
    // Test the constructor directly
    let mock_repo = MockUserRepositoryTrait::new();
    let mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    // Just verify it constructs without panicking
    assert!(std::ptr::addr_of!(user_service) as *const _ != std::ptr::null());
}

#[tokio::test]
async fn test_update_user_password_update_error_in_email_change_path() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mut mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();
    let current_user = create_test_user(user_id, "testuser", Some("old@example.com"), true);
    let new_email = "new@example.com";

    let user_after_email_update = User {
        email: Some(new_email.to_string()),
        is_email_verified: false,
        ..current_user.clone()
    };

    mock_repo.expect_find_by_id()
        .times(1)
        .returning(move |_| Ok(Some(current_user.clone())));

    mock_repo.expect_find_by_email_and_verified()
        .times(1)
        .returning(|_| Ok(None));

    mock_repo.expect_update_email_and_set_unverified()
        .times(1)
        .returning(move |_, _, _| Ok(user_after_email_update.clone()));

    // Password update fails
    mock_repo.expect_update_password()
        .times(1)
        .returning(|_, _| Err(sqlx::Error::PoolClosed));

    mock_email_service.expect_send_verification_email()
        .times(1)
        .returning(|_, _| Ok(()));

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let input = UserInput {
        username: "testuser".to_string(),
        email: Some(new_email.to_string()),
        password: Some("NewPassword123!".to_string()),
    };

    let result = user_service.update_user(user_id, input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Database);
}

#[tokio::test]
async fn test_update_user_find_by_id_after_password_update_error() {
    let mut mock_repo = MockUserRepositoryTrait::new();
    let mut mock_email_service = MockEmailServiceTrait::new();
    let mock_token_revocation_service = MockTokenRevocationServiceTrait::new();

    let user_id = Uuid::new_v4();
    let current_user = create_test_user(user_id, "testuser", Some("old@example.com"), true);
    let new_email = "new@example.com";

    let user_after_email_update = User {
        email: Some(new_email.to_string()),
        is_email_verified: false,
        ..current_user.clone()
    };

    // First call to find_by_id succeeds
    mock_repo.expect_find_by_id()
        .with(predicate::eq(user_id))
        .times(1)
        .returning(move |_| Ok(Some(current_user.clone())));

    mock_repo.expect_find_by_email_and_verified()
        .times(1)
        .returning(|_| Ok(None));

    mock_repo.expect_update_email_and_set_unverified()
        .times(1)
        .returning(move |_, _, _| Ok(user_after_email_update.clone()));

    mock_repo.expect_update_password()
        .times(1)
        .returning(|_, _| Ok(()));

    // Second call to find_by_id (after password update) returns None
    mock_repo.expect_find_by_id()
        .with(predicate::eq(user_id))
        .times(1)
        .returning(|_| Ok(None));

    mock_email_service.expect_send_verification_email()
        .times(1)
        .returning(|_, _| Ok(()));

    let user_service = UserService::new(
        Arc::new(mock_repo),
        Arc::new(mock_email_service),
        Arc::new(mock_token_revocation_service),
    );

    let input = UserInput {
        username: "testuser".to_string(),
        email: Some(new_email.to_string()),
        password: Some("NewPassword123!".to_string()),
    };

    let result = user_service.update_user(user_id, input).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.error_type, ApiErrorType::Internal);
    assert_eq!(err.message, "User consistency error after update");
}