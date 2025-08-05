use validator::ValidationError;
use regex::Regex;
use lazy_static::lazy_static;

lazy_static! {
    static ref EMAIL_REGEX: Regex = Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
    ).unwrap();

    pub static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();

    // PASSWORD_REGEX removed as we will implement checks manually
}

pub fn validate_length(value: &str, min: usize, max: usize) -> Result<(), ValidationError> {
    if value.len() < min || value.len() > max {
        // Use to_owned to convert the formatted string to an owned String
        return Err(ValidationError {
            code: "length".into(),
            message: Some(format!("Length must be between {min} and {max} characters").to_owned().into()),
            params: Default::default(),
        });
    }
    Ok(())
}

pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    if !EMAIL_REGEX.is_match(email) {
        return Err(ValidationError::new("Invalid email format"));
    }
    Ok(())
}

pub fn validate_username(username: &str) -> Result<(), ValidationError> {
    // Length check (e.g., 3-50 characters)
    if username.len() < 3 || username.len() > 50 {
        return Err(ValidationError {
            code: "length".into(),
            message: Some("Username must be between 3 and 50 characters".into()),
            params: Default::default(),
        });
    }
    if !USERNAME_REGEX.is_match(username) {
        return Err(ValidationError {
            code: "format".into(),
            message: Some("Username can only contain letters, numbers, underscores, and hyphens".into()),
            params: Default::default(),
        });
    }
    Ok(())
}

pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::new("Password must be at least 8 characters long."));
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(ValidationError::new("Password must contain at least one lowercase letter."));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError::new("Password must contain at least one uppercase letter."));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(ValidationError::new("Password must contain at least one number."));
    }
    if !password.chars().any(|c| "@$!%*?&".contains(c)) {
        return Err(ValidationError::new("Password must contain at least one special character (@$!%*?&)."));
    }
    Ok(())
}

pub fn sanitize_string(input: &str) -> String {
    ammonia::clean(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_length() {
        assert!(validate_length("test", 1, 10).is_ok());
        assert!(validate_length("test", 5, 10).is_err());
        assert!(validate_length("test", 1, 3).is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name+tag@example.co.uk").is_ok());
        assert!(validate_email("invalid.email").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("user_name").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("ab").is_err()); // Too short
        assert!(validate_username("user name").is_err()); // Contains space
        assert!(validate_username("user@name").is_err()); // Contains special char
    }

    #[test]
    fn test_validate_password_strength() {
        assert!(validate_password_strength("Password1!").is_ok());
        assert!(validate_password_strength("StrongP@ss1").is_ok());
        assert!(validate_password_strength("weak").is_err());
        assert!(validate_password_strength("NoSpecial1").is_err());
        assert!(validate_password_strength("no_upper1!").is_err());
        assert!(validate_password_strength("NOLOWER1!").is_err());
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("<script>alert('xss')</script>"), "");
        assert_eq!(sanitize_string("Normal text"), "Normal text");
        assert_eq!(
            sanitize_string("<p>Safe <b>HTML</b></p>"),
            "<p>Safe <b>HTML</b></p>"
        );
    }
    // Additional comprehensive tests from tests/validation_utils_tests.rs
    
    #[test]
    fn test_validate_length_comprehensive() {
        // Valid lengths
        assert!(validate_length("hello", 3, 10).is_ok());
        assert!(validate_length("test", 4, 4).is_ok());
        
        // Too short
        assert!(validate_length("hi", 3, 10).is_err());
        
        // Too long
        assert!(validate_length("this is way too long", 3, 10).is_err());
    }

    #[test]
    fn test_validate_email_comprehensive() {
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
    fn test_validate_username_comprehensive() {
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
    fn test_validate_password_strength_comprehensive() {
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
    fn test_sanitize_string_comprehensive() {
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
}
