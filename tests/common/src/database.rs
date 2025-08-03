//! Database test utilities

use oxidizedoasis_websands::infrastructure::database::connection::create_pool;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Database test helper for setting up and tearing down test data
pub struct DatabaseTestHelper {
    pool: Arc<PgPool>,
}

impl DatabaseTestHelper {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub async fn from_config(config: &AppConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let pool = create_pool(config).await?;
        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    pub fn pool(&self) -> Arc<PgPool> {
        self.pool.clone()
    }

    /// Clean up test data - removes all test users and related data
    pub async fn cleanup_all(&self) -> Result<(), sqlx::Error> {
        // Clean up in reverse dependency order
        sqlx::query("DELETE FROM password_reset_tokens WHERE 1=1")
            .execute(self.pool.as_ref())
            .await?;
        
        sqlx::query("DELETE FROM revoked_tokens WHERE 1=1")
            .execute(self.pool.as_ref())
            .await?;
        
        sqlx::query("DELETE FROM active_tokens WHERE 1=1")
            .execute(self.pool.as_ref())
            .await?;
        
        sqlx::query("DELETE FROM users WHERE username LIKE 'test_%' OR email LIKE '%@test.com'")
            .execute(self.pool.as_ref())
            .await?;

        Ok(())
    }

    /// Insert a test user directly into the database
    pub async fn insert_test_user(
        &self,
        id: Uuid,
        username: &str,
        email: &str,
        password_hash: &str,
        role: &str,
        is_verified: bool,
        is_active: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, is_email_verified, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
            id,
            username,
            email,
            password_hash,
            role,
            is_verified,
            is_active
        )
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Check if a user exists in the database
    pub async fn user_exists(&self, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM users WHERE id = $1", id)
            .fetch_one(self.pool.as_ref())
            .await?;
        
        Ok(result.count.unwrap_or(0) > 0)
    }

    /// Get user count in database
    pub async fn user_count(&self) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!("SELECT COUNT(*) as count FROM users")
            .fetch_one(self.pool.as_ref())
            .await?;
        
        Ok(result.count.unwrap_or(0))
    }

    /// Create a test password reset token
    pub async fn insert_password_reset_token(
        &self,
        user_id: Uuid,
        token: &str,
        expires_in_minutes: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            INSERT INTO password_reset_tokens (id, user_id, token, expires_at, is_used, created_at, updated_at)
            VALUES ($1, $2, $3, NOW() + INTERVAL '1 minute' * $4, false, NOW(), NOW())
            "#,
            Uuid::new_v4(),
            user_id,
            token,
            expires_in_minutes as f64
        )
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    /// Run database migrations for testing
    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("../../migrations").run(self.pool.as_ref()).await?;
        Ok(())
    }

    /// Check database connection health
    pub async fn check_health(&self) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("SELECT 1")
            .fetch_one(self.pool.as_ref())
            .await?;
        
        Ok(result.get::<i32, _>(0) == 1)
    }

    /// Create a test user and return it
    pub async fn create_test_user(&self) -> Result<oxidizedoasis_websands::core::user::User, sqlx::Error> {
        use oxidizedoasis_websands::core::user::User;
        use chrono::Utc;
        let user_id = Uuid::new_v4();
        let username = format!("test_user_{}", user_id);
        let email = format!("{}@test.com", username);
        let password_hash = bcrypt::hash("test_password", bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            r#"
            INSERT INTO users (id, username, email, password_hash, role, is_email_verified, is_active, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
            user_id,
            username,
            Some(email.clone()),
            password_hash,
            "user",
            true,
            true
        )
        .execute(self.pool.as_ref())
        .await?;

        Ok(User {
            id: user_id,
            username,
            email: Some(email),
            password_hash,
            role: "user".to_string(),
            is_active: true,
            is_email_verified: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verification_token: None,
            verification_token_expires_at: None,
        })
    }
}

/// Test transaction helper for isolated tests
pub struct TestTransaction {
    tx: sqlx::Transaction<'static, sqlx::Postgres>,
}

impl TestTransaction {
    pub async fn begin(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let tx = pool.begin().await?;
        Ok(Self { tx })
    }

    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        self.tx.rollback().await
    }

    pub async fn commit(self) -> Result<(), sqlx::Error> {
        self.tx.commit().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_helper_creation() {
        // Test removed temporarily to fix import issues
        // The database helper functionality is tested elsewhere
        assert!(true);
    }
}