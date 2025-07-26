// src/core/user/repository.rs
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use super::model::{User, PasswordResetToken, NewUser}; // Added NewUser
use crate::common::validation::UserInput;
use log::{error, info, debug};
use mockall::automock;

#[automock]
#[async_trait]
pub trait UserRepositoryTrait: Send + Sync {
    async fn create_user_with_details(&self, user_input: &UserInput, password_hash: String, verification_token: String) -> Result<User, sqlx::Error>;
    async fn create_user(&self, new_user: NewUser) -> Result<User, sqlx::Error>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error>;
    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error>;
    async fn verify_email(&self, token: &str) -> Result<Option<Uuid>, sqlx::Error>;
    async fn find_by_verification_token(&self, token: &str) -> Result<Option<User>, sqlx::Error>;
    async fn update_email_and_set_unverified(&self, user_id: Uuid, new_email: &str, verification_token: &str) -> Result<User, sqlx::Error>;
    async fn update(&self, id: Uuid, user_input: &UserInput, password_hash: Option<String>) -> Result<User, sqlx::Error>;
    async fn update_role(&self, user_id: Uuid, new_role: &str) -> Result<Option<User>, sqlx::Error>;
    async fn update_username(&self, user_id: Uuid, new_username: &str) -> Result<Option<User>, sqlx::Error>;
    async fn update_status(&self, user_id: Uuid, is_active: bool) -> Result<Option<User>, sqlx::Error>;
    async fn update_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), sqlx::Error>;
    async fn update_verification_token(&self, user_id: Uuid, token: &str) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error>;
    async fn check_email_verified(&self, username: &str) -> Result<bool, sqlx::Error>;
    async fn create_password_reset_token(&self, user_id: Uuid) -> Result<PasswordResetToken, sqlx::Error>;
    async fn verify_reset_token(&self, token: &str) -> Result<Option<PasswordResetToken>, sqlx::Error>;
    async fn mark_reset_token_used(&self, token: &str) -> Result<bool, sqlx::Error>;
    async fn find_all(&self) -> Result<Vec<User>, sqlx::Error>;
    async fn find_by_email_and_verified(&self, email: &str) -> Result<Option<User>, sqlx::Error>;
    #[cfg(test)]
    async fn clear_all(&self) -> Result<(), sqlx::Error>;
}

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn create_user_with_details(
        &self,
        user_input: &UserInput,
        password_hash: String,
        verification_token: String,
    ) -> Result<User, sqlx::Error> {
        debug!("Creating new user with details for username: {}", user_input.username);

        let verification_token_expires_at = Utc::now() + Duration::hours(24);
        let now = Utc::now();

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                id,
                username,
                email,
                password_hash,
                is_email_verified,
                verification_token,
                verification_token_expires_at,
                created_at,
                updated_at,
                role,
                is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
            Uuid::new_v4(),
            user_input.username,
            user_input.email,
            password_hash,
            false, // is_email_verified
            Some(verification_token),
            Some(verification_token_expires_at),
            now, // created_at
            now, // updated_at
            "user", // Default role
            true    // Default is_active
        )
        .fetch_one(&self.pool)
        .await;

        match &user {
            Ok(u) => info!("Successfully created user with details, id: {}", u.id),
            Err(e) => error!("Failed to create user with details: {}", e),
        }
        user
    }

    async fn create_user(&self, new_user: NewUser) -> Result<User, sqlx::Error> {
        debug!("Creating new user from NewUser struct for username: {}", new_user.username);
        let now = Utc::now();

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (
                id,
                username,
                email,
                password_hash,
                is_email_verified,
                verification_token,
                verification_token_expires_at,
                created_at,
                updated_at,
                role,
                is_active
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
            Uuid::new_v4(),
            new_user.username,
            new_user.email,
            new_user.password_hash,
            new_user.is_email_verified,
            new_user.verification_token,
            new_user.verification_token_expires_at,
            now, // created_at
            now, // updated_at
            new_user.role,
            true // Default is_active to true for new users
        )
        .fetch_one(&self.pool)
        .await;

        match &user {
            Ok(u) => info!("Successfully created user from NewUser, id: {}", u.id),
            Err(e) => error!("Failed to create user from NewUser: {}", e),
        }
        user
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        debug!("Looking up user by username: {}", username);

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            debug!("Found user: {}", u.id);
        }
        user
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        debug!("Looking up user by id: {}", id);

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            debug!("Found user: {}", u.username);
        }
        user
    }

    async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        debug!("Looking up user by email: {}", email);

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE email = $1
            "#,
            Some(email) // Ensure email is passed as Option<String> if the DB column is nullable, or String if not.
                        // Assuming email in users table is nullable based on UserInput.email being Option<String>
                        // and NewUser.email being Option<String>.
                        // If the DB column is NOT NULL, this should be just `email`.
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            debug!("Found user: {}", u.id);
        }
        user
    }

    async fn find_by_verification_token(&self, token: &str) -> Result<Option<User>, sqlx::Error> {
        debug!("Looking up user by verification token");

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE verification_token = $1
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            debug!("Found user by verification token: {}", u.id);
        }
        user
    }
    
    async fn verify_email(&self, token: &str) -> Result<Option<Uuid>, sqlx::Error> {
        debug!("Attempting to verify email with token");

        let result = sqlx::query!(
            r#"
            UPDATE users
            SET is_email_verified = TRUE,
                verification_token = NULL,
                verification_token_expires_at = NULL,
                updated_at = $1
            WHERE verification_token = $2
              AND verification_token_expires_at > CURRENT_TIMESTAMP
            RETURNING id
            "#,
            Utc::now(),
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(ref r) = result {
            info!("Successfully verified email for user: {}", r.id);
        } else {
            debug!("No user found with the provided verification token or token expired");
        }
        Ok(result.map(|r| r.id))
    }

    async fn update(
        &self,
        id: Uuid,
        user_input: &UserInput,
        password_hash: Option<String>,
    ) -> Result<User, sqlx::Error> {
        debug!("Updating user: {}", id);

        let current_user = self.find_by_id(id)
            .await?
            .ok_or_else(|| {
                error!("User not found for update: {}", id);
                sqlx::Error::RowNotFound
            })?;

        let final_password_hash = password_hash.unwrap_or_else(|| current_user.password_hash.clone());
        let final_username = user_input.username.clone(); 
        let final_email = user_input.email.clone().or_else(|| current_user.email.clone());

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET username = $1,
                email = $2,
                password_hash = $3,
                updated_at = $4
            WHERE id = $5
            RETURNING *
            "#,
            final_username,
            final_email, 
            final_password_hash,
            Utc::now(),
            id
        )
        .fetch_one(&self.pool)
        .await;

        if let Ok(ref u) = user {
            info!("Successfully updated user: {}", u.id);
        } else if let Err(ref e) = user {
            error!("Failed to update user {}: {}", id, e);
        }
        user
    }

    async fn update_role(
        &self,
        user_id: Uuid,
        new_role: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        debug!("Updating role for user {}: {}", user_id, new_role);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET role = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
            new_role,
            Utc::now(),
            user_id
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            info!("Successfully updated role for user: {}", u.id);
        }
        user
    }

    async fn update_username(
        &self,
        user_id: Uuid,
        new_username: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        debug!("Updating username for user {}: {}", user_id, new_username);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET username = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
            new_username,
            Utc::now(),
            user_id
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            info!("Successfully updated username for user: {}", u.id);
        }
        user
    }

    async fn update_status(
        &self,
        user_id: Uuid,
        is_active: bool,
    ) -> Result<Option<User>, sqlx::Error> {
        debug!("Updating status for user {}: active={}", user_id, is_active);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET is_active = $1,
                updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
            is_active,
            Utc::now(),
            user_id
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            info!("Successfully updated status for user: {}", u.id);
        }
        user
    }
    
    async fn update_password(&self, user_id: Uuid, password_hash: &str) -> Result<(), sqlx::Error> {
        debug!("Updating password for user: {}", user_id);

        sqlx::query!(
            r#"
            UPDATE users
            SET password_hash = $1,
                updated_at = $2
            WHERE id = $3
            "#,
            password_hash,
            Utc::now(),
            user_id
        )
        .execute(&self.pool)
        .await?;

        info!("Successfully updated password for user: {}", user_id);
        Ok(())
    }

    async fn update_verification_token(
        &self,
        user_id: Uuid,
        token: &str,
    ) -> Result<(), sqlx::Error> {
        debug!("Updating verification token for user: {}", user_id);
        let expires_at = Utc::now() + Duration::hours(24);

        sqlx::query!(
            r#"
            UPDATE users
            SET verification_token = $1,
                verification_token_expires_at = $2,
                updated_at = $3
            WHERE id = $4
            "#,
            token,
            expires_at,
            Utc::now(),
            user_id
        )
        .execute(&self.pool)
        .await?;

        info!("Successfully updated verification token for user: {}", user_id);
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        debug!("Attempting to delete user: {}", id);

        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            info!("Successfully deleted user: {}", id);
        } else {
            debug!("No user found to delete with id: {}", id);
        }
        Ok(deleted)
    }

    async fn check_email_verified(&self, username: &str) -> Result<bool, sqlx::Error> {
        debug!("Checking email verification status for user: {}", username);

        let result = sqlx::query!(
            r#"
            SELECT is_email_verified
            FROM users
            WHERE username = $1
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.map_or(false, |r| r.is_email_verified))
    }

    async fn create_password_reset_token(&self, user_id: Uuid) -> Result<PasswordResetToken, sqlx::Error> {
        debug!("Creating password reset token for user: {}", user_id);
        let token = Uuid::new_v4().to_string();
        let expires_at = Utc::now() + Duration::hours(1);
        let now = Utc::now();

        let reset_token = sqlx::query_as!(
            PasswordResetToken,
            r#"
            INSERT INTO password_reset_tokens (
                id, user_id, token, expires_at, is_used, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
            Uuid::new_v4(),
            user_id,
            token,
            expires_at,
            false,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await;

        match &reset_token {
            Ok(_t) => info!("Created password reset token for user: {}", user_id),
            Err(e) => error!("Failed to create password reset token: {}", e),
        }
        reset_token
    }

    async fn verify_reset_token(&self, token: &str) -> Result<Option<PasswordResetToken>, sqlx::Error> {
        debug!("Verifying password reset token");

        let reset_token = sqlx::query_as!(
            PasswordResetToken,
            r#"
            SELECT *
            FROM password_reset_tokens
            WHERE token = $1
              AND expires_at > CURRENT_TIMESTAMP
              AND is_used = false
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref t)) = reset_token {
            debug!("Found valid reset token for user: {}", t.user_id);
        }
        reset_token
    }

    async fn mark_reset_token_used(&self, token: &str) -> Result<bool, sqlx::Error> {
        debug!("Marking reset token as used");
        let result = sqlx::query!(
            r#"
            UPDATE password_reset_tokens
            SET is_used = true,
                updated_at = $1
            WHERE token = $2
              AND expires_at > CURRENT_TIMESTAMP
              AND is_used = false
            "#,
            Utc::now(),
            token
        )
        .execute(&self.pool)
        .await?;

        let marked = result.rows_affected() > 0;
        if marked {
            info!("Successfully marked reset token as used");
        } else {
            debug!("No valid reset token found to mark as used");
        }
        Ok(marked)
    }
    
    async fn find_all(&self) -> Result<Vec<User>, sqlx::Error> {
        debug!("Fetching all users");
        
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    #[cfg(test)]
    async fn clear_all(&self) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM password_reset_tokens").execute(&self.pool).await?;
        sqlx::query!("DELETE FROM users").execute(&self.pool).await?;
        Ok(())
    }

    async fn update_email_and_set_unverified(
        &self,
        user_id: Uuid,
        new_email: &str,
        verification_token: &str,
    ) -> Result<User, sqlx::Error> {
        debug!(
            "Updating email for user {} to {} and setting as unverified",
            user_id, new_email
        );
        let verification_token_expires_at = Utc::now() + Duration::hours(24);
        let now = Utc::now();

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET email = $1,
                is_email_verified = $2,
                verification_token = $3,
                verification_token_expires_at = $4,
                updated_at = $5
            WHERE id = $6
            RETURNING *
            "#,
            new_email,
            false, // is_email_verified
            Some(verification_token.to_string()),
            Some(verification_token_expires_at),
            now, // updated_at
            user_id
        )
        .fetch_one(&self.pool)
        .await;

        match &user {
            Ok(u) => info!("Successfully updated email for user: {}", u.id),
            Err(e) => error!("Failed to update email for user {}: {}", user_id, e),
        }
        user
    }

    async fn find_by_email_and_verified(
        &self,
        email: &str,
    ) -> Result<Option<User>, sqlx::Error> {
        debug!(
            "Looking up user by email {} and verified status",
            email
        );

        let user = sqlx::query_as!(
            User,
            r#"
            SELECT *
            FROM users
            WHERE email = $1 AND is_email_verified = TRUE
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(ref u)) = user {
            debug!("Found verified user by email: {}", u.id);
        }
        user
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::validation::UserInput; // For creating users
    use crate::infrastructure::config::app_config::DatabaseConfig; // For test DB connection
    use std::sync::Arc;
    use tokio; // For async tests

    // Helper to set up a test database and repository
    async fn setup_test_repository() -> UserRepository {
        // Replace with your actual test DB configuration logic
        // This often involves environment variables or a test config file
        // For simplicity, using a hardcoded fallback (NOT recommended for real projects)
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://testuser:testpassword@localhost:5433/testdb".to_string());
        
        let config = DatabaseConfig {
            url: db_url,
            max_connections: 5,
        };
        let pool = config.get_pool().await.expect("Failed to create test DB pool");
        
        // Optional: Run migrations if your test DB is ephemeral or needs schema setup
        // sqlx::migrate!("./migrations").run(&pool).await.expect("Failed to run migrations on test DB");

        UserRepository::new(pool)
    }

    // Helper to create a unique user for testing
    fn create_test_user_input(username_suffix: &str, email_suffix: &str) -> UserInput {
        UserInput {
            username: format!("testuser_{}", username_suffix),
            email: Some(format!("test_{}@example.com", email_suffix)),
            password: Some("Password123!".to_string()),
        }
    }
    
    #[tokio::test]
    #[ignore] // Skip by default since it requires a test database
    async fn test_update_email_and_set_unverified_success() {
        let repo = setup_test_repository().await;
        repo.clear_all().await.unwrap(); // Clear previous test data

        let user_input = create_test_user_input("update_email", "update_email");
        let initial_user = repo.create_user_with_details(
            &user_input, 
            "hashed_password".to_string(), 
            "initial_token".to_string()
        ).await.unwrap();

        let new_email = "new.updated.email@example.com";
        let new_token = "new_verification_token";
        
        let updated_user_res = repo.update_email_and_set_unverified(initial_user.id, new_email, new_token).await;
        assert!(updated_user_res.is_ok());
        let updated_user = updated_user_res.unwrap();

        assert_eq!(updated_user.email.as_deref(), Some(new_email));
        assert_eq!(updated_user.is_email_verified, false);
        assert_eq!(updated_user.verification_token.as_deref(), Some(new_token));
        
        let expected_expiry = Utc::now() + Duration::hours(24);
        assert!(updated_user.verification_token_expires_at.is_some());
        let diff = expected_expiry - updated_user.verification_token_expires_at.unwrap();
        assert!(diff.num_seconds().abs() < 60, "Expiry time should be roughly 24 hours from now"); // Allow 1 min difference

        // Fetch directly and assert
        let fetched_user = repo.find_by_id(initial_user.id).await.unwrap().unwrap();
        assert_eq!(fetched_user.email.as_deref(), Some(new_email));
        assert_eq!(fetched_user.is_email_verified, false);
        assert_eq!(fetched_user.verification_token.as_deref(), Some(new_token));
        assert!(fetched_user.verification_token_expires_at.is_some());
        let diff_fetched = expected_expiry - fetched_user.verification_token_expires_at.unwrap();
        assert!(diff_fetched.num_seconds().abs() < 60);
        
        repo.clear_all().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Skip by default since it requires a test database
    async fn test_find_by_email_and_verified_found() {
        let repo = setup_test_repository().await;
        repo.clear_all().await.unwrap();

        let email_verified = "verified.user@example.com";
        // Create a verified user directly using NewUser for more control
        let new_user_verified = NewUser {
            username: "verified_user".to_string(),
            email: Some(email_verified.to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            role: "user".to_string(),
        };
        let created_verified_user = repo.create_user(new_user_verified).await.unwrap();

        let result = repo.find_by_email_and_verified(email_verified).await;
        assert!(result.is_ok());
        let found_user_opt = result.unwrap();
        assert!(found_user_opt.is_some());
        let found_user = found_user_opt.unwrap();
        assert_eq!(found_user.id, created_verified_user.id);
        assert_eq!(found_user.email.as_deref(), Some(email_verified));
        assert!(found_user.is_email_verified);
        
        repo.clear_all().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Skip by default since it requires a test database
    async fn test_find_by_email_and_verified_not_found_if_unverified() {
        let repo = setup_test_repository().await;
        repo.clear_all().await.unwrap();

        let email_unverified = "unverified.user@example.com";
        let new_user_unverified = NewUser {
            username: "unverified_user".to_string(),
            email: Some(email_unverified.to_string()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: false, // Key part of this test
            verification_token: Some("some_token".to_string()),
            verification_token_expires_at: Some(Utc::now() + Duration::hours(24)),
            role: "user".to_string(),
        };
        repo.create_user(new_user_unverified).await.unwrap();

        let result = repo.find_by_email_and_verified(email_unverified).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        
        repo.clear_all().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Skip by default since it requires a test database
    async fn test_find_by_email_and_verified_not_found_for_nonexistent_email() {
        let repo = setup_test_repository().await;
        repo.clear_all().await.unwrap(); // Ensure clean state

        let result = repo.find_by_email_and_verified("nosuch.user@example.com").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        
        // No cleanup needed as no user was created
    }
}
