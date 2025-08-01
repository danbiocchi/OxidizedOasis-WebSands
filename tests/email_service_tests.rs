//! Tests for src/core/email/service.rs - Email service functionality
//!
//! This test file covers the EmailService implementation including
//! email sending, template integration, SMTP configuration, and error handling.

use oxidizedoasis_websands::core::email::service::{EmailService, EmailServiceTrait, MockEmailServiceTrait};
use std::env;
use std::sync::Mutex;
use tokio;

// Use a global mutex to ensure tests that modify environment variables don't interfere with each other
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod email_service_tests {
    use super::*;

    /// Helper function to set up test environment variables
    fn setup_test_env() {
        env::set_var("SMTP_USERNAME", "test@example.com");
        env::set_var("SMTP_PASSWORD", "test_password");
        env::set_var("SMTP_SERVER", "smtp.example.com");
        env::set_var("FROM_EMAIL", "noreply@example.com");
        env::set_var("APP_NAME", "TestApp");
        env::set_var("EMAIL_FROM_NAME", "Test Application");
        env::set_var("EMAIL_VERIFICATION_SUBJECT", "Verify Your Email - TestApp");
        env::set_var("EMAIL_PASSWORD_RESET_SUBJECT", "Reset Your Password - TestApp");
        env::set_var("ENVIRONMENT", "development");
        env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
        env::set_var("PRODUCTION_URL", "https://example.com");
    }

    /// Helper function to clean up test environment variables
    fn cleanup_test_env() {
        let vars_to_remove = [
            "SMTP_USERNAME", "SMTP_PASSWORD", "SMTP_SERVER", "FROM_EMAIL",
            "APP_NAME", "EMAIL_FROM_NAME", "EMAIL_VERIFICATION_SUBJECT", 
            "EMAIL_PASSWORD_RESET_SUBJECT", "ENVIRONMENT", "DEVELOPMENT_URL", "PRODUCTION_URL"
        ];
        
        for var in &vars_to_remove {
            env::remove_var(var);
        }
    }

    /// Test EmailService creation with valid environment variables
    #[test]
    fn test_email_service_creation() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        setup_test_env();
        
        let email_service = EmailService::new();
        
        // Note: We can't directly access private fields, but we can test that creation succeeds
        // The fact that new() doesn't panic means all required env vars were present
        
        cleanup_test_env();
    }

    /// Test EmailService creation fails with missing environment variables
    #[test]
    #[should_panic(expected = "SMTP_USERNAME must be set")]
    fn test_email_service_creation_missing_smtp_username() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        // Set all required environment variables EXCEPT SMTP_USERNAME
        env::set_var("SMTP_PASSWORD", "test_password");
        env::set_var("SMTP_SERVER", "smtp.example.com");
        env::set_var("FROM_EMAIL", "noreply@example.com");
        env::set_var("APP_NAME", "TestApp");
        env::set_var("EMAIL_FROM_NAME", "Test Application");
        env::set_var("EMAIL_VERIFICATION_SUBJECT", "Verify Your Email");
        env::set_var("EMAIL_PASSWORD_RESET_SUBJECT", "Reset Password");
        env::set_var("ENVIRONMENT", "development");
        env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
        env::set_var("PRODUCTION_URL", "https://example.com");
        // SMTP_USERNAME is intentionally not set
        
        let _email_service = EmailService::new();
    }

    /// Test EmailService creation fails with missing SMTP_PASSWORD
    #[test]
    #[should_panic(expected = "SMTP_PASSWORD must be set")]
    fn test_email_service_creation_missing_smtp_password() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        
        // Set all other required vars EXCEPT SMTP_PASSWORD
        env::set_var("SMTP_USERNAME", "test@example.com");
        env::set_var("SMTP_SERVER", "smtp.example.com");
        env::set_var("FROM_EMAIL", "noreply@example.com");
        env::set_var("APP_NAME", "TestApp");
        env::set_var("EMAIL_FROM_NAME", "Test Application");
        env::set_var("EMAIL_VERIFICATION_SUBJECT", "Verify Your Email");
        env::set_var("EMAIL_PASSWORD_RESET_SUBJECT", "Reset Password");
        env::set_var("ENVIRONMENT", "development");
        env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
        env::set_var("PRODUCTION_URL", "https://example.com");
        // SMTP_PASSWORD is intentionally not set
        
        let _email_service = EmailService::new();
    }

    /// Test EmailService creation fails with missing SMTP_SERVER
    #[test]
    #[should_panic(expected = "SMTP_SERVER must be set")]
    fn test_email_service_creation_missing_smtp_server() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        // Set all required environment variables EXCEPT SMTP_SERVER
        env::set_var("SMTP_USERNAME", "test@example.com");
        env::set_var("SMTP_PASSWORD", "test_password");
        env::set_var("FROM_EMAIL", "noreply@example.com");
        env::set_var("APP_NAME", "TestApp");
        env::set_var("EMAIL_FROM_NAME", "Test Application");
        env::set_var("EMAIL_VERIFICATION_SUBJECT", "Verify Your Email");
        env::set_var("EMAIL_PASSWORD_RESET_SUBJECT", "Reset Password");
        env::set_var("ENVIRONMENT", "development");
        env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
        env::set_var("PRODUCTION_URL", "https://example.com");
        // SMTP_SERVER is intentionally not set
        
        let _email_service = EmailService::new();
    }

    /// Test environment detection with development environment
    #[test]
    fn test_environment_detection_development() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        
        // We can't directly test the private get_base_url function,
        // but we test that EmailService creation succeeds with development config
        setup_test_env();
        env::set_var("ENVIRONMENT", "development");
        env::set_var("DEVELOPMENT_URL", "http://localhost:3000");
        
        let _email_service = EmailService::new();
        
        cleanup_test_env();
    }

    /// Test environment detection with production environment
    #[test]
    fn test_environment_detection_production() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        
        // We can't directly test the private get_base_url function,
        // but we test that EmailService creation succeeds with production config
        setup_test_env();
        env::set_var("ENVIRONMENT", "production");
        env::set_var("PRODUCTION_URL", "https://myapp.com");
        
        let _email_service = EmailService::new();
        
        cleanup_test_env();
    }

    /// Test MockEmailServiceTrait functionality for verification email success
    #[tokio::test]
    async fn test_mock_email_service_verification_email_success() {
        let mut mock_service = MockEmailServiceTrait::new();
        
        mock_service
            .expect_send_verification_email()
            .with(
                mockall::predicate::eq("test@example.com"),
                mockall::predicate::eq("verification_token_123")
            )
            .times(1)
            .returning(|_, _| Ok(()));

        // Test the mock
        let result = mock_service
            .send_verification_email("test@example.com", "verification_token_123")
            .await;
        
        assert!(result.is_ok());
    }

    /// Test MockEmailServiceTrait functionality for password reset email success
    #[tokio::test]
    async fn test_mock_email_service_password_reset_email_success() {
        let mut mock_service = MockEmailServiceTrait::new();
        
        mock_service
            .expect_send_password_reset_email()
            .with(
                mockall::predicate::eq("user@example.com"),
                mockall::predicate::eq("reset_token_456")
            )
            .times(1)
            .returning(|_, _| Ok(()));

        // Test the mock
        let result = mock_service
            .send_password_reset_email("user@example.com", "reset_token_456")
            .await;
        
        assert!(result.is_ok());
    }

    /// Test MockEmailServiceTrait error simulation for verification email
    #[tokio::test]
    async fn test_mock_email_service_verification_email_failure() {
        let mut mock_service = MockEmailServiceTrait::new();
        
        mock_service
            .expect_send_verification_email()
            .with(
                mockall::predicate::eq("test@example.com"),
                mockall::predicate::eq("invalid_token")
            )
            .times(1)
            .returning(|_, _| Err("SMTP connection failed".into()));

        // Test the mock failure
        let result = mock_service
            .send_verification_email("test@example.com", "invalid_token")
            .await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SMTP connection failed"));
    }

    /// Test MockEmailServiceTrait error simulation for password reset email
    #[tokio::test]
    async fn test_mock_email_service_password_reset_email_failure() {
        let mut mock_service = MockEmailServiceTrait::new();
        
        mock_service
            .expect_send_password_reset_email()
            .with(
                mockall::predicate::eq("user@example.com"),
                mockall::predicate::eq("invalid_reset_token")
            )
            .times(1)
            .returning(|_, _| Err("Email server unreachable".into()));

        // Test the mock failure
        let result = mock_service
            .send_password_reset_email("user@example.com", "invalid_reset_token")
            .await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Email server unreachable"));
    }

    /// Test MockEmailServiceTrait with multiple calls
    #[tokio::test]
    async fn test_mock_email_service_multiple_calls() {
        let mut mock_service = MockEmailServiceTrait::new();
        
        // Set up expectations for multiple calls
        mock_service
            .expect_send_verification_email()
            .times(2)
            .returning(|_, _| Ok(()));
            
        mock_service
            .expect_send_password_reset_email()
            .times(1)
            .returning(|_, _| Ok(()));

        // Make multiple calls
        let result1 = mock_service
            .send_verification_email("user1@example.com", "token1")
            .await;
        let result2 = mock_service
            .send_verification_email("user2@example.com", "token2")
            .await;
        let result3 = mock_service
            .send_password_reset_email("user3@example.com", "reset_token")
            .await;
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
    }

    /// Test MockEmailServiceTrait with specific parameter matching
    #[tokio::test]
    async fn test_mock_email_service_parameter_matching() {
        let mut mock_service = MockEmailServiceTrait::new();
        
        // Set up specific parameter expectations
        mock_service
            .expect_send_verification_email()
            .with(
                mockall::predicate::function(|email: &str| email.contains("@example.com")),
                mockall::predicate::function(|token: &str| token.starts_with("verify_"))
            )
            .times(1)
            .returning(|_, _| Ok(()));

        // Test with matching parameters
        let result = mock_service
            .send_verification_email("test@example.com", "verify_abc123")
            .await;
        
        assert!(result.is_ok());
    }

    /// Test that EmailService clone works properly
    #[test]
    fn test_email_service_clone() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cleanup_test_env();
        setup_test_env();
        
        let email_service = EmailService::new();
        let cloned_service = email_service.clone();
        
        // Both instances should be valid - if clone didn't work properly,
        // one of these would fail
        drop(email_service);
        drop(cloned_service);
        
        cleanup_test_env();
    }
}
