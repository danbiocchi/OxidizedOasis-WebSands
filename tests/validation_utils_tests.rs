//! Unit tests for validation utilities

use oxidizedoasis_websands::common::utils::validation::{
    validate_length, validate_email, validate_username, validate_password_strength, sanitize_string
};

#[test]
fn test_validate_length() {
    // Valid lengths
    assert!(validate_length("hello", 3, 10).is_ok());
    assert!(validate_length("test", 4, 4).is_ok());
    
    // Too short
    assert!(validate_length("hi", 3, 10).is_err());
    
    // Too long
    assert!(validate_length("this is way too long", 3, 10).is_err());
}

#[test]
fn test_validate_email() {
    // Valid emails
    assert!(validate_email("test@example.com").is_ok());
    assert!(validate_email("user.name@domain.co.uk").is_ok());
    assert!(validate_email("valid+email@test.org").is_ok());
    
    // Invalid emails
    assert!(validate_email("invalid-email").is_err());
    assert!(validate_email("@domain.com").is_err());
    assert!(validate_email("user@").is_err());
    assert!(validate_email("").is_err());
}

#[test]
fn test_validate_username() {
    // Valid usernames
    assert!(validate_username("validuser123").is_ok());
    assert!(validate_username("user_name").is_ok());
    assert!(validate_username("TestUser").is_ok());
    
    // Invalid usernames
    assert!(validate_username("").is_err()); // Empty
    assert!(validate_username("us").is_err()); // Too short
    assert!(validate_username("user@name").is_err()); // Special chars
    assert!(validate_username("user name").is_err()); // Spaces
}

#[test]
fn test_validate_password_strength() {
    // Strong passwords
    assert!(validate_password_strength("MyPassword123!").is_ok());
    assert!(validate_password_strength("AnotherGood1@").is_ok());
    
    // Weak passwords
    assert!(validate_password_strength("weak").is_err()); // Too short
    assert!(validate_password_strength("password").is_err()); // No uppercase/numbers
    assert!(validate_password_strength("PASSWORD123").is_err()); // No lowercase
    assert!(validate_password_strength("Password").is_err()); // No numbers
}

#[test]
fn test_sanitize_string() {
    // HTML sanitization (ammonia preserves safe content and whitespace)
    assert_eq!(sanitize_string("  hello  "), "  hello  ");
    assert_eq!(sanitize_string("test\n\r\t"), "test\n\n\t");
    
    // HTML-like content - dangerous scripts removed
    assert_eq!(sanitize_string("<script>alert('xss')</script>"), "");
    
    // Normal text should be preserved
    assert_eq!(sanitize_string("Hello World!"), "Hello World!");
    
    // Safe HTML should be preserved
    assert_eq!(sanitize_string("<p>Safe <b>HTML</b></p>"), "<p>Safe <b>HTML</b></p>");
}

#[test]
fn test_sanitize_string_empty() {
    assert_eq!(sanitize_string(""), "");
    assert_eq!(sanitize_string("   "), "   "); // whitespace is preserved
}