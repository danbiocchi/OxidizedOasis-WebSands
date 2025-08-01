use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)] // Added Clone
pub struct PasswordResetToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub is_used: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetVerify {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct PasswordResetSubmit {
    pub token: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)] // Added Clone
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_email_verified: bool,
    pub verification_token: Option<String>,
    pub verification_token_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub role: String,
    pub is_active: bool,
}

// Struct for updating an existing user
#[derive(Debug, Deserialize, Serialize, Clone, Default)] // Added Default for easier test setup
pub struct UserUpdate {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub is_email_verified: Option<bool>,
    pub verification_token: Option<Option<String>>, // Use Option<Option<String>> to allow setting to Some(None) to clear the token
    pub verification_token_expires_at: Option<Option<DateTime<Utc>>>, // Use Option<Option<DateTime<Utc>>> to allow setting to Some(None) to clear the expiration
    pub role: Option<String>,
    pub is_active: Option<bool>,
}


// Struct for creating a new user
#[derive(Debug, Deserialize, Serialize, Clone)] // Added Clone, Serialize, Deserialize for flexibility
pub struct NewUser {
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_email_verified: bool,
    pub role: String,
    pub verification_token: Option<String>,
    pub verification_token_expires_at: Option<DateTime<Utc>>,
    // is_active is typically true by default, can be omitted or set by repository
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub is_email_verified: bool,
    pub created_at: DateTime<Utc>,
    pub role: String,
    pub is_active: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
            is_email_verified: user.is_email_verified,
            created_at: user.created_at,
            role: user.role,
            is_active: user.is_active,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_user_to_user_response_conversion() {
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        
        let user = User {
            id: user_id,
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: Some("verify_token".to_string()),
            verification_token_expires_at: Some(Utc::now()),
            created_at,
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };

        let user_response: UserResponse = user.into();

        assert_eq!(user_response.id, user_id);
        assert_eq!(user_response.username, "testuser");
        assert_eq!(user_response.email, Some("test@example.com".to_string()));
        assert_eq!(user_response.is_email_verified, true);
        assert_eq!(user_response.created_at, created_at);
        assert_eq!(user_response.role, "user");
        assert_eq!(user_response.is_active, true);
    }

    #[test]
    fn test_user_to_user_response_conversion_with_none_email() {
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        
        let user = User {
            id: user_id,
            username: "testuser2".to_string(),
            email: None,
            password_hash: "hashed_password".to_string(),
            is_email_verified: false,
            verification_token: None,
            verification_token_expires_at: None,
            created_at,
            updated_at: Utc::now(),
            role: "admin".to_string(),
            is_active: false,
        };

        let user_response: UserResponse = user.into();

        assert_eq!(user_response.id, user_id);
        assert_eq!(user_response.username, "testuser2");
        assert_eq!(user_response.email, None);
        assert_eq!(user_response.is_email_verified, false);
        assert_eq!(user_response.created_at, created_at);
        assert_eq!(user_response.role, "admin");
        assert_eq!(user_response.is_active, false);
    }

    #[test]
    fn test_password_reset_token_creation() {
        let token_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        let expires_at = Utc::now();
        
        let token = PasswordResetToken {
            id: token_id,
            user_id,
            token: "reset_token_123".to_string(),
            expires_at,
            is_used: false,
            created_at,
            updated_at: created_at,
        };

        assert_eq!(token.id, token_id);
        assert_eq!(token.user_id, user_id);
        assert_eq!(token.token, "reset_token_123");
        assert_eq!(token.expires_at, expires_at);
        assert_eq!(token.is_used, false);
        assert_eq!(token.created_at, created_at);
        assert_eq!(token.updated_at, created_at);
    }

    #[test]
    fn test_password_reset_request_creation() {
        let request = PasswordResetRequest {
            email: "user@example.com".to_string(),
        };

        assert_eq!(request.email, "user@example.com");
    }

    #[test]
    fn test_password_reset_verify_creation() {
        let verify = PasswordResetVerify {
            token: "verify_token_456".to_string(),
        };

        assert_eq!(verify.token, "verify_token_456");
    }

    #[test]
    fn test_password_reset_submit_creation() {
        let submit = PasswordResetSubmit {
            token: "submit_token_789".to_string(),
            new_password: "new_secure_password".to_string(),
            confirm_password: "new_secure_password".to_string(),
        };

        assert_eq!(submit.token, "submit_token_789");
        assert_eq!(submit.new_password, "new_secure_password");
        assert_eq!(submit.confirm_password, "new_secure_password");
    }

    #[test]
    fn test_user_creation() {
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        
        let user = User {
            id: user_id,
            username: "newuser".to_string(),
            email: Some("new@example.com".to_string()),
            password_hash: "secure_hash".to_string(),
            is_email_verified: true,
            verification_token: Some("token".to_string()),
            verification_token_expires_at: Some(Utc::now()),
            created_at,
            updated_at: created_at,
            role: "user".to_string(),
            is_active: true,
        };

        assert_eq!(user.id, user_id);
        assert_eq!(user.username, "newuser");
        assert_eq!(user.email, Some("new@example.com".to_string()));
        assert_eq!(user.password_hash, "secure_hash");
        assert_eq!(user.is_email_verified, true);
        assert_eq!(user.role, "user");
        assert_eq!(user.is_active, true);
    }

    #[test]
    fn test_user_update_default() {
        let update = UserUpdate::default();

        assert_eq!(update.username, None);
        assert_eq!(update.email, None);
        assert_eq!(update.password_hash, None);
        assert_eq!(update.is_email_verified, None);
        assert_eq!(update.verification_token, None);
        assert_eq!(update.verification_token_expires_at, None);
        assert_eq!(update.role, None);
        assert_eq!(update.is_active, None);
    }

    #[test]
    fn test_user_update_with_values() {
        let update = UserUpdate {
            username: Some("updated_user".to_string()),
            email: Some("updated@example.com".to_string()),
            password_hash: Some("new_hash".to_string()),
            is_email_verified: Some(true),
            verification_token: Some(Some("new_token".to_string())),
            verification_token_expires_at: Some(Some(Utc::now())),
            role: Some("admin".to_string()),
            is_active: Some(false),
        };

        assert_eq!(update.username, Some("updated_user".to_string()));
        assert_eq!(update.email, Some("updated@example.com".to_string()));
        assert_eq!(update.password_hash, Some("new_hash".to_string()));
        assert_eq!(update.is_email_verified, Some(true));
        assert!(update.verification_token.is_some());
        assert!(update.verification_token_expires_at.is_some());
        assert_eq!(update.role, Some("admin".to_string()));
        assert_eq!(update.is_active, Some(false));
    }

    #[test]
    fn test_new_user_creation() {
        let new_user = NewUser {
            username: "brandnew".to_string(),
            email: Some("brandnew@example.com".to_string()),
            password_hash: "brand_new_hash".to_string(),
            is_email_verified: false,
            role: "user".to_string(),
            verification_token: Some("brand_new_token".to_string()),
            verification_token_expires_at: Some(Utc::now()),
        };

        assert_eq!(new_user.username, "brandnew");
        assert_eq!(new_user.email, Some("brandnew@example.com".to_string()));
        assert_eq!(new_user.password_hash, "brand_new_hash");
        assert_eq!(new_user.is_email_verified, false);
        assert_eq!(new_user.role, "user");
        assert!(new_user.verification_token.is_some());
        assert!(new_user.verification_token_expires_at.is_some());
    }

    #[test]
    fn test_user_clone() {
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        
        let user = User {
            id: user_id,
            username: "cloneable".to_string(),
            email: Some("clone@example.com".to_string()),
            password_hash: "hash".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at,
            updated_at: created_at,
            role: "user".to_string(),
            is_active: true,
        };

        let cloned_user = user.clone();

        assert_eq!(user.id, cloned_user.id);
        assert_eq!(user.username, cloned_user.username);
        assert_eq!(user.email, cloned_user.email);
        assert_eq!(user.is_active, cloned_user.is_active);
    }

    #[test]
    fn test_password_reset_token_clone() {
        let token_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let created_at = Utc::now();
        
        let token = PasswordResetToken {
            id: token_id,
            user_id,
            token: "cloneable_token".to_string(),
            expires_at: created_at,
            is_used: false,
            created_at,
            updated_at: created_at,
        };

        let cloned_token = token.clone();

        assert_eq!(token.id, cloned_token.id);
        assert_eq!(token.user_id, cloned_token.user_id);
        assert_eq!(token.token, cloned_token.token);
        assert_eq!(token.is_used, cloned_token.is_used);
    }
}
