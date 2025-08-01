//! Tests for src/core/user/model.rs - User data structures and serialization
//! 
//! This test file covers all the user-related data structures including User, 
//! NewUser, UserUpdate, UserResponse, and password reset structures.

use oxidizedoasis_websands::core::user::model::*;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};
use serde_json;

#[cfg(test)]
mod user_model_tests {
    use super::*;

    /// Test User struct creation and field access
    #[test]
    fn test_user_struct_creation() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let user = User {
            id: user_id,
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: now,
            updated_at: now,
            role: "user".to_string(),
            is_active: true,
        };

        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, Some("test@example.com".to_string()));
        assert_eq!(user.password_hash, "hashed_password");
        assert!(user.is_email_verified);
        assert_eq!(user.role, "user");
        assert!(user.is_active);
        assert_eq!(user.created_at, now);
        assert_eq!(user.updated_at, now);
    }

    /// Test User struct with verification token
    #[test]
    fn test_user_with_verification_token() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::days(1);
        
        let user = User {
            id: user_id,
            username: "unverified_user".to_string(),
            email: Some("unverified@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: false,
            verification_token: Some("verification_token_123".to_string()),
            verification_token_expires_at: Some(expires_at),
            created_at: now,
            updated_at: now,
            role: "user".to_string(),
            is_active: true,
        };

        assert!(!user.is_email_verified);
        assert_eq!(user.verification_token, Some("verification_token_123".to_string()));
        assert_eq!(user.verification_token_expires_at, Some(expires_at));
    }

    /// Test User serialization and deserialization
    #[test]
    fn test_user_serialization() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let user = User {
            id: user_id,
            username: "serialize_test".to_string(),
            email: Some("serialize@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: now,
            updated_at: now,
            role: "admin".to_string(),
            is_active: true,
        };

        // Test serialization
        let serialized = serde_json::to_string(&user).expect("Should serialize");
        assert!(serialized.contains("serialize_test"));
        assert!(serialized.contains("serialize@example.com"));
        assert!(serialized.contains("admin"));

        // Test deserialization
        let deserialized: User = serde_json::from_str(&serialized).expect("Should deserialize");
        assert_eq!(deserialized.id, user.id);
        assert_eq!(deserialized.username, user.username);
        assert_eq!(deserialized.email, user.email);
        assert_eq!(deserialized.role, user.role);
    }

    /// Test User clone functionality
    #[test]
    fn test_user_clone() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let original_user = User {
            id: user_id,
            username: "clone_test".to_string(),
            email: Some("clone@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: now,
            updated_at: now,
            role: "user".to_string(),
            is_active: true,
        };

        let cloned_user = original_user.clone();
        
        assert_eq!(original_user.id, cloned_user.id);
        assert_eq!(original_user.username, cloned_user.username);
        assert_eq!(original_user.email, cloned_user.email);
        assert_eq!(original_user.role, cloned_user.role);
    }

    /// Test NewUser struct creation and serialization
    #[test]
    fn test_new_user_creation() {
        let now = Utc::now();
        let expires_at = now + Duration::days(1);
        
        let new_user = NewUser {
            username: "newuser".to_string(),
            email: Some("newuser@example.com".to_string()),
            password_hash: "new_hashed_password".to_string(),
            is_email_verified: false,
            role: "user".to_string(),
            verification_token: Some("new_token_456".to_string()),
            verification_token_expires_at: Some(expires_at),
        };

        assert_eq!(new_user.username, "newuser");
        assert_eq!(new_user.email, Some("newuser@example.com".to_string()));
        assert!(!new_user.is_email_verified);
        assert_eq!(new_user.role, "user");
        assert_eq!(new_user.verification_token, Some("new_token_456".to_string()));

        // Test serialization
        let serialized = serde_json::to_string(&new_user).expect("Should serialize");
        assert!(serialized.contains("newuser"));
        assert!(serialized.contains("newuser@example.com"));
    }

    /// Test NewUser clone functionality
    #[test]
    fn test_new_user_clone() {
        let new_user = NewUser {
            username: "cloneuser".to_string(),
            email: Some("clone@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            role: "admin".to_string(),
            verification_token: None,
            verification_token_expires_at: None,
        };

        let cloned = new_user.clone();
        assert_eq!(new_user.username, cloned.username);
        assert_eq!(new_user.email, cloned.email);
        assert_eq!(new_user.role, cloned.role);
    }

    /// Test UserUpdate struct creation and default behavior
    #[test]
    fn test_user_update_creation() {
        let user_update = UserUpdate {
            username: Some("updated_username".to_string()),
            email: Some("updated@example.com".to_string()),
            password_hash: None,
            is_email_verified: Some(true),
            verification_token: Some(None), // Clear the token
            verification_token_expires_at: Some(None), // Clear the expiration
            role: Some("admin".to_string()),
            is_active: Some(false),
        };

        assert_eq!(user_update.username, Some("updated_username".to_string()));
        assert_eq!(user_update.email, Some("updated@example.com".to_string()));
        assert_eq!(user_update.password_hash, None);
        assert_eq!(user_update.is_email_verified, Some(true));
        assert_eq!(user_update.verification_token, Some(None));
        assert_eq!(user_update.role, Some("admin".to_string()));
        assert_eq!(user_update.is_active, Some(false));
    }

    /// Test UserUpdate default implementation
    #[test]
    fn test_user_update_default() {
        let user_update = UserUpdate::default();
        
        assert_eq!(user_update.username, None);
        assert_eq!(user_update.email, None);
        assert_eq!(user_update.password_hash, None);
        assert_eq!(user_update.is_email_verified, None);
        assert_eq!(user_update.verification_token, None);
        assert_eq!(user_update.verification_token_expires_at, None);
        assert_eq!(user_update.role, None);
        assert_eq!(user_update.is_active, None);
    }

    /// Test UserResponse creation and From trait implementation
    #[test]
    fn test_user_response_from_user() {
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        
        let user = User {
            id: user_id,
            username: "response_test".to_string(),
            email: Some("response@example.com".to_string()),
            password_hash: "secret_hash".to_string(), // This should not appear in response
            is_email_verified: true,
            verification_token: Some("secret_token".to_string()), // This should not appear in response
            verification_token_expires_at: Some(now + Duration::hours(1)),
            created_at: now,
            updated_at: now,
            role: "user".to_string(),
            is_active: true,
        };

        let response: UserResponse = user.into();
        
        assert_eq!(response.id, user_id);
        assert_eq!(response.username, "response_test");
        assert_eq!(response.email, Some("response@example.com".to_string()));
        assert!(response.is_email_verified);
        assert_eq!(response.created_at, now);
        assert_eq!(response.role, "user");
        assert!(response.is_active);

        // Test serialization of response (should not contain sensitive data)
        let serialized = serde_json::to_string(&response).expect("Should serialize");
        assert!(!serialized.contains("secret_hash"));
        assert!(!serialized.contains("secret_token"));
        assert!(serialized.contains("response_test"));
        assert!(serialized.contains("response@example.com"));
    }

    /// Test PasswordResetToken struct
    #[test]
    fn test_password_reset_token() {
        let token_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let now = Utc::now();
        let expires_at = now + Duration::hours(1);
        
        let reset_token = PasswordResetToken {
            id: token_id,
            user_id,
            token: "reset_token_789".to_string(),
            expires_at,
            is_used: false,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(reset_token.id, token_id);
        assert_eq!(reset_token.user_id, user_id);
        assert_eq!(reset_token.token, "reset_token_789");
        assert_eq!(reset_token.expires_at, expires_at);
        assert!(!reset_token.is_used);

        // Test clone functionality
        let cloned = reset_token.clone();
        assert_eq!(reset_token.id, cloned.id);
        assert_eq!(reset_token.token, cloned.token);
    }

    /// Test password reset request structs
    #[test]
    fn test_password_reset_request_structs() {
        // Test PasswordResetRequest
        let reset_request = PasswordResetRequest {
            email: "reset@example.com".to_string(),
        };
        assert_eq!(reset_request.email, "reset@example.com");

        // Test PasswordResetVerify
        let reset_verify = PasswordResetVerify {
            token: "verify_token_123".to_string(),
        };
        assert_eq!(reset_verify.token, "verify_token_123");

        // Test PasswordResetSubmit
        let reset_submit = PasswordResetSubmit {
            token: "submit_token_456".to_string(),
            new_password: "new_secure_password".to_string(),
            confirm_password: "new_secure_password".to_string(),
        };
        assert_eq!(reset_submit.token, "submit_token_456");
        assert_eq!(reset_submit.new_password, "new_secure_password");
        assert_eq!(reset_submit.confirm_password, "new_secure_password");
    }

    /// Test password reset structs deserialization
    #[test]
    fn test_password_reset_deserialization() {
        // Test PasswordResetRequest deserialization
        let json = r#"{"email": "test@example.com"}"#;
        let request: PasswordResetRequest = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(request.email, "test@example.com");

        // Test PasswordResetVerify deserialization
        let json = r#"{"token": "abc123"}"#;
        let verify: PasswordResetVerify = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(verify.token, "abc123");

        // Test PasswordResetSubmit deserialization
        let json = r#"{"token": "xyz789", "new_password": "newpass", "confirm_password": "newpass"}"#;
        let submit: PasswordResetSubmit = serde_json::from_str(json).expect("Should deserialize");
        assert_eq!(submit.token, "xyz789");
        assert_eq!(submit.new_password, "newpass");
        assert_eq!(submit.confirm_password, "newpass");
    }

    /// Test edge cases and special scenarios
    #[test]
    fn test_edge_cases() {
        // Test User with None email
        let user_with_no_email = User {
            id: Uuid::new_v4(),
            username: "no_email_user".to_string(),
            email: None,
            password_hash: "hash".to_string(),
            is_email_verified: false,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };

        assert_eq!(user_with_no_email.email, None);
        
        let response: UserResponse = user_with_no_email.into();
        assert_eq!(response.email, None);

        // Test NewUser with None email
        let new_user_no_email = NewUser {
            username: "new_no_email".to_string(),
            email: None,
            password_hash: "hash".to_string(),
            is_email_verified: false,
            role: "user".to_string(),
            verification_token: None,
            verification_token_expires_at: None,
        };

        assert_eq!(new_user_no_email.email, None);
    }
}