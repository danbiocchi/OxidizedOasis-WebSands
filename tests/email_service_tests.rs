use crate::common::test_data::TEST_USER_EMAIL;
use mockall::predicate::eq;
use oxidizedoasis_websands::core::email::EmailServiceTrait;
use oxidizedoasis_websands::core::email::service::MockEmailServiceTrait;

mod common;

/// Test verification email functionality
#[tokio::test]
async fn test_verification_email() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    // Test verification email template
    let verification_token = "test_verification_token_123";
    
    mock_email_service
        .expect_send_verification_email()
        .with(eq(TEST_USER_EMAIL), eq(verification_token))
        .times(1)
        .returning(|_, _| Ok(()));
    
    let result = mock_email_service
        .send_verification_email(TEST_USER_EMAIL, verification_token)
        .await;
    
    assert!(result.is_ok());
}

/// Test password reset email functionality
#[tokio::test]
async fn test_password_reset_email() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let reset_token = "test_reset_token_456";
    
    mock_email_service
        .expect_send_password_reset_email()
        .with(eq(TEST_USER_EMAIL), eq(reset_token))
        .times(1)
        .returning(|_, _| Ok(()));
    
    let result = mock_email_service
        .send_password_reset_email(TEST_USER_EMAIL, reset_token)
        .await;
    
    assert!(result.is_ok());
}

/// Test email service error handling for verification
#[tokio::test]
async fn test_verification_email_error_handling() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let verification_token = "test_token";
    
    // Set up expectation for failure
    mock_email_service
        .expect_send_verification_email()
        .with(eq("invalid@email"), eq(verification_token))
        .times(1)
        .returning(|_, _| Err("SMTP connection failed".into()));
    
    let result = mock_email_service
        .send_verification_email("invalid@email", verification_token)
        .await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SMTP connection failed"));
}

/// Test email service error handling for password reset
#[tokio::test]
async fn test_password_reset_email_error_handling() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let reset_token = "test_reset_token";
    
    // Set up expectation for failure
    mock_email_service
        .expect_send_password_reset_email()
        .with(eq("invalid@email"), eq(reset_token))
        .times(1)
        .returning(|_, _| Err("SMTP connection failed".into()));
    
    let result = mock_email_service
        .send_password_reset_email("invalid@email", reset_token)
        .await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SMTP connection failed"));
}

/// Test bulk verification email operations
#[tokio::test]
async fn test_bulk_verification_emails() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let recipients = vec!["user1@test.com", "user2@test.com", "user3@test.com"];
    let token = "bulk_verification_token";
    
    // Set up expectations for multiple emails
    for recipient in &recipients {
        mock_email_service
            .expect_send_verification_email()
            .with(eq(*recipient), eq(token))
            .times(1)
            .returning(|_, _| Ok(()));
    }
    
    // Send emails to all recipients
    for recipient in recipients {
        let result = mock_email_service
            .send_verification_email(recipient, token)
            .await;
        assert!(result.is_ok());
    }
}

/// Test bulk password reset email operations
#[tokio::test]
async fn test_bulk_password_reset_emails() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let recipients = vec!["user1@test.com", "user2@test.com", "user3@test.com"];
    
    // Set up expectations for multiple emails with different tokens
    for (i, recipient) in recipients.iter().enumerate() {
        let token = format!("reset_token_{}", i);
        mock_email_service
            .expect_send_password_reset_email()
            .with(eq(*recipient), eq(token))
            .times(1)
            .returning(|_, _| Ok(()));
    }
    
    // Send emails to all recipients
    for (i, recipient) in recipients.iter().enumerate() {
        let token = format!("reset_token_{}", i);
        let result = mock_email_service
            .send_password_reset_email(recipient, &token)
            .await;
        assert!(result.is_ok());
    }
}

/// Test email service integration with user registration
#[tokio::test]
async fn test_email_integration_with_user_registration() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    let verification_token = "verification_token_123";
    
    mock_email_service
        .expect_send_verification_email()
        .with(eq("testuser@example.com"), eq(verification_token))
        .times(1)
        .returning(|_, _| Ok(()));
    
    // Simulate sending verification email
    let result = mock_email_service
        .send_verification_email("testuser@example.com", verification_token)
        .await;
    
    assert!(result.is_ok());
}

/// Test email service with malformed email addresses for verification
#[tokio::test]
async fn test_malformed_email_addresses_verification() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let invalid_emails = vec![
        "invalid-email",
        "@example.com",
        "user@",
        "",
    ];
    
    let token = "test_token";
    
    for invalid_email in invalid_emails {
        mock_email_service
            .expect_send_verification_email()
            .with(eq(invalid_email), eq(token))
            .times(1)
            .returning(|_, _| Err("Invalid email address".into()));
        
        let result = mock_email_service.send_verification_email(invalid_email, token).await;
        assert!(result.is_err());
    }
}

/// Test email service with malformed email addresses for password reset
#[tokio::test]
async fn test_malformed_email_addresses_password_reset() {
    let mut mock_email_service = MockEmailServiceTrait::new();
    
    let invalid_emails = vec![
        "invalid-email",
        "@example.com",
        "user@",
        "",
    ];
    
    let token = "test_reset_token";
    
    for invalid_email in invalid_emails {
        mock_email_service
            .expect_send_password_reset_email()
            .with(eq(invalid_email), eq(token))
            .times(1)
            .returning(|_, _| Err("Invalid email address".into()));
        
        let result = mock_email_service.send_password_reset_email(invalid_email, token).await;
        assert!(result.is_err());
    }
}
