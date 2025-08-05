use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError, ValidationErrorsKind};
 
 
use super::password::validate_password;
use actix_web::web;
use super::super::utils::validation::USERNAME_REGEX;

fn validate_username_regex(username: &str) -> Result<(), validator::ValidationError> {
    if !USERNAME_REGEX.is_match(username) {
        return Err(validator::ValidationError::new("regex"));
    }
    Ok(())
}

#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct UserInput {
    #[validate(length(min = 3, max = 50), custom(function = "validate_username_regex"))]
    pub username: String,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(custom(function = "validate_password"))]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct LoginInput {
    #[validate(length(min = 3, max = 50), custom(function = "validate_username_regex"))]
    pub username: String,
    #[validate(custom(function = "validate_password"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize, Validate, Clone)]
pub struct RegisterInput {
    #[validate(length(min = 3, max = 50), custom(function = "validate_username_regex"))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    pub password: String,
    pub password_confirm: String,
}

fn validate_passwords_match(input: &RegisterInput) -> Result<(), ValidationError> {
    if input.password != input.password_confirm {
        let mut err = ValidationError::new("passwords_must_match");
        err.message = Some("Passwords must match".into());
        return Err(err);
    }
    Ok(())
}

pub fn sanitize_input(input: &str) -> String {
    let stripped = ammonia::clean(input);
    stripped.trim().to_string()
}

// Helper function to recursively collect all validation errors
fn collect_errors_recursive(
    errors_kind: ValidationErrorsKind,
    all_errors: &mut Vec<ValidationError>,
) {
    match errors_kind {
        ValidationErrorsKind::Field(field_vec) => {
            all_errors.extend(field_vec);
        }
        ValidationErrorsKind::Struct(struct_errors_box) => {
            // struct_errors_box is Box<validator::ValidationErrors>
            // .errors() returns &IndexMap<Cow<'static, str>, ValidationErrorsKind>
            for kind_ref in struct_errors_box.errors().values() { 
                collect_errors_recursive(kind_ref.clone(), all_errors);
            }
        }
        ValidationErrorsKind::List(list_errors_map) => {
            // list_errors_map is IndexMap<usize, Box<validator::ValidationErrors>>
            // Each value in list_errors_map is Box<ValidationErrors>
            for (_index, nested_errors_box) in list_errors_map {
                // nested_errors_box is Box<validator::ValidationErrors>
                for kind_ref in nested_errors_box.errors().values() { 
                    collect_errors_recursive(kind_ref.clone(), all_errors);
                }
            }
        }
    }
}

pub fn validate_and_sanitize_user_input(input: UserInput) -> Result<UserInput, Vec<ValidationError>> {
    let mut errors = Vec::new();

    if let Err(validation_errors) = input.validate() {
        for (_field_name, kind) in validation_errors.into_errors() {
            collect_errors_recursive(kind, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(UserInput {
            username: sanitize_input(&input.username),
            email: input.email.map(|e| sanitize_input(&e)),
            password: input.password,
        })
    } else {
        Err(errors)
    }
}

pub fn validate_and_sanitize_login_input(input: LoginInput) -> Result<LoginInput, Vec<ValidationError>> {
    let mut errors = Vec::new();
    if let Err(validation_errors) = input.validate() {
        for (_field_name, kind) in validation_errors.into_errors() {
            collect_errors_recursive(kind, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(LoginInput {
            username: sanitize_input(&input.username),
            password: input.password,
        })
    } else {
        Err(errors)
    }
}

pub fn validate_and_sanitize_register_input(input: RegisterInput) -> Result<RegisterInput, Vec<ValidationError>> {
    let mut errors = Vec::<ValidationError>::new();

    let sanitized_username = sanitize_input(&input.username);
    let sanitized_email = sanitize_input(&input.email);

    let validation_target = RegisterInput {
        username: sanitized_username.clone(),
        email: sanitized_email.clone(),
        password: input.password.clone(), 
        password_confirm: input.password_confirm.clone(),
    };

    if let Err(validation_errors) = validation_target.validate() {
        for (_field_name, kind) in validation_errors.into_errors() {
            collect_errors_recursive(kind, &mut errors);
        }
    }
    
    if let Err(e) = validate_password(&input.password) {
        errors.push(e); 
    }

    if let Err(e) = validate_passwords_match(&input) {
        errors.push(e);
    }

    if errors.is_empty() {
        Ok(RegisterInput {
            username: sanitized_username,
            email: sanitized_email,
            password: input.password, 
            password_confirm: input.password_confirm,
        })
    } else {
        Err(errors)
    }
}


#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct TokenQuery {
    #[validate(length(min = 1))]
    token: String,
}

impl TokenQuery {
    pub fn token(&self) -> &str {
        &self.token
    }
}

impl TryFrom<web::Query<TokenQuery>> for TokenQuery {
    type Error = actix_web::Error;

    fn try_from(query: web::Query<TokenQuery>) -> Result<Self, Self::Error> {
        if let Err(e) = query.validate() {
            return Err(actix_web::error::ErrorBadRequest(e));
        }
        Ok(query.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    fn new_register_input(username: &str, email: &str, password: &str, password_confirm: &str) -> RegisterInput {
        RegisterInput {
            username: username.to_string(),
            email: email.to_string(),
            password: password.to_string(),
            password_confirm: password_confirm.to_string(),
        }
    }

    #[test]
    fn test_sanitize_input_basic() {
        assert_eq!(sanitize_input("  <script>alert('xss')</script> Test  "), "Test");
        assert_eq!(sanitize_input("  Normal Text "), "Normal Text");
    }

    #[test]
    fn test_validate_passwords_match_valid() {
        let input = new_register_input("user", "user@example.com", "P@ssw0rd1", "P@ssw0rd1");
        assert!(validate_passwords_match(&input).is_ok());
    }

    #[test]
    fn test_validate_passwords_match_invalid() {
        let input = new_register_input("user", "user@example.com", "P@ssw0rd1", "DifferentP@ssw0rd1");
        let result = validate_passwords_match(&input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "passwords_must_match");
        assert_eq!(err.message.unwrap(), "Passwords must match");
    }

    #[test]
    fn test_register_input_valid() {
        let input = new_register_input("testuser", "test@example.com", "ValidP@ss1", "ValidP@ss1");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
    }

    #[test]
    fn test_register_input_username_too_short() {
        let input = new_register_input("u", "test@example.com", "ValidP@ss1", "ValidP@ss1");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "length" && e.params.contains_key(&std::borrow::Cow::Borrowed("value")) && e.params[&std::borrow::Cow::Borrowed("value")] == serde_json::json!("u")), "Expected username length error for 'u'");
    }
    
    #[test]
    fn test_register_input_username_too_long() {
        let long_username = "u".repeat(51);
        let input = new_register_input(&long_username, "test@example.com", "ValidP@ss1", "ValidP@ss1");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "length" && e.params.contains_key(&std::borrow::Cow::Borrowed("value")) && e.params[&std::borrow::Cow::Borrowed("value")] == serde_json::json!(long_username)), "Expected username length error for long username");
    }

    #[test]
    fn test_register_input_username_invalid_chars() {
        let input = new_register_input("test user!", "test@example.com", "ValidP@ss1", "ValidP@ss1");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "regex"), "Expected username regex error"); // Changed "USERNAME_REGEX" to "regex"
    }


    #[test]
    fn test_register_input_invalid_email() {
        let input = new_register_input("testuser", "invalidemail", "ValidP@ss1", "ValidP@ss1");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "email"));
    }

    #[test]
    fn test_register_input_invalid_password_short() {
        let input = new_register_input("testuser", "test@example.com", "short", "short");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "Password must be between 8 and 100 characters long"));
    }
    
    #[test]
    fn test_register_input_password_no_uppercase() {
        let input = new_register_input("testuser", "test@example.com", "noupper1!", "noupper1!");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "Password must contain at least one uppercase letter"));
    }

    #[test]
    fn test_register_input_passwords_do_not_match() {
        let input = new_register_input("testuser", "test@example.com", "ValidP@ss1", "DifferentP@ss2");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "passwords_must_match"));
    }

    #[test]
    fn test_register_input_sanitization() {
        // Test case 1: Sanitization results in an invalid empty username
        let input_malicious_username = new_register_input("  <script>alert('xss')</script>  ", "  test@example.com  ", "ValidP@ss1", "ValidP@ss1");
        let result1 = validate_and_sanitize_register_input(input_malicious_username.clone());
        assert!(result1.is_err(), "Expected Err for username that sanitizes to empty, got: {:?}", result1);
        if let Err(errors) = result1 {
            assert!(errors.iter().any(|e| e.code == "length" && e.params.get("value").map_or(false, |v| v == "")), "Expected length error for empty sanitized username");
            assert!(errors.iter().any(|e| e.code == "regex" && e.params.get("value").map_or(false, |v| v == "")), "Expected regex error for empty sanitized username");
        }

        // Test case 2: Sanitization cleans benign input correctly and passes validation
        let input_benign_spaces = new_register_input("  testuser  ", "  test@example.com  ", "ValidP@ss1", "ValidP@ss1");
        let result2 = validate_and_sanitize_register_input(input_benign_spaces.clone());
        assert!(result2.is_ok(), "Expected Ok for benign input with spaces, got: {:?}", result2.err());
        if let Ok(sanitized) = result2 {
            assert_eq!(sanitized.username, "testuser");
            assert_eq!(sanitized.email, "test@example.com");
        }
    }

    #[test]
    fn test_register_input_multiple_errors() {
        let input = new_register_input("u", "invalid", "short", "mismatch");
        let result = validate_and_sanitize_register_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "length"));
        assert!(errors.iter().any(|e| e.code == "email"));
        assert!(errors.iter().any(|e| e.code == "Password must be between 8 and 100 characters long"));
        assert!(errors.iter().any(|e| e.code == "passwords_must_match"));
        assert!(errors.len() >= 4); 
    }

    fn new_login_input(username: &str, password: &str) -> LoginInput {
        LoginInput {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    #[test]
    fn test_login_input_valid() {
        let input = new_login_input("testuser", "ValidP@ss1");
        let result = validate_and_sanitize_login_input(input);
        assert!(result.is_ok(), "Login input should be valid: {:?}", result.err());
    }

    #[test]
    fn test_login_input_invalid_username_format() {
        let input = new_login_input("test user", "ValidP@ss1"); 
        let result = validate_and_sanitize_login_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "regex"), "Expected username regex error"); // Changed "USERNAME_REGEX" to "regex"
    }
    
    #[test]
    fn test_login_input_invalid_password() {
        let input = new_login_input("testuser", "weak");
        let result = validate_and_sanitize_login_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "Password must be between 8 and 100 characters long"));
    }

    fn new_user_input(username: &str, email: Option<&str>, password: Option<&str>) -> UserInput {
        UserInput {
            username: username.to_string(),
            email: email.map(String::from),
            password: password.map(String::from),
        }
    }

    #[test]
    fn test_user_input_valid_all_fields() {
        let input = new_user_input("testuser", Some("test@example.com"), Some("ValidP@ss1"));
        let result = validate_and_sanitize_user_input(input);
        assert!(result.is_ok(), "User input should be valid: {:?}", result.err());
    }

    #[test]
    fn test_user_input_valid_optional_missing() {
        let input = new_user_input("testuser", None, None);
        let result = validate_and_sanitize_user_input(input);
        assert!(result.is_ok(), "User input with optional missing should be valid: {:?}", result.err());
    }
    
    #[test]
    fn test_user_input_invalid_username_regex() {
        let input = new_user_input("test user", Some("test@example.com"), Some("ValidP@ss1"));
        let result = validate_and_sanitize_user_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "regex"), "Expected username regex error"); // Changed "USERNAME_REGEX" to "regex"
    }

    #[test]
    fn test_user_input_invalid_email() {
        let input = new_user_input("testuser", Some("invalid"), Some("ValidP@ss1"));
        let result = validate_and_sanitize_user_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "email"));
    }

    #[test]
    fn test_user_input_invalid_password() {
        let input = new_user_input("testuser", Some("test@example.com"), Some("weak"));
        let result = validate_and_sanitize_user_input(input);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.code == "Password must be between 8 and 100 characters long"));
    }

    #[test]
    fn test_token_query_valid() {
        let tq = TokenQuery { token: "a_valid_token".to_string() };
        assert!(tq.validate().is_ok());
    }

    #[test]
    fn test_token_query_invalid_empty() {
        let tq = TokenQuery { token: "".to_string() };
        assert!(tq.validate().is_err());
        let errors = tq.validate().unwrap_err();
        assert!(errors.field_errors().contains_key("token"));
    }
}
