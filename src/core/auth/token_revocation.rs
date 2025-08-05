use sqlx::{PgPool, Error as SqlxError};
use uuid::Uuid;
use chrono::{DateTime, Utc}; // Duration might not be needed by the trait itself
use log::{debug, error, info, warn};
use crate::core::auth::jwt::TokenType;
use crate::core::auth::active_token::ActiveTokenServiceTrait; // Use the trait
use mockall::automock;
use async_trait::async_trait;
use std::sync::Arc;

/// Represents a revoked token in the database
#[derive(Debug, Clone)] // Added Clone
pub struct RevokedToken {
    pub id: Uuid,
    pub jti: String,
    pub user_id: Uuid,
    pub token_type: String,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[automock]
#[async_trait]
pub trait TokenRevocationServiceTrait: Send + Sync {
    // Method to set active_token_service, if needed by the trait.
    // Or, ActiveTokenServiceTrait could be a parameter to new/constructor.
    // For now, let's assume the concrete type handles its own dependencies.
    // The trait will only expose the public contract.
    // fn set_active_token_service(&mut self, service: Arc<dyn ActiveTokenServiceTrait>);

    async fn is_token_revoked(&self, jti: &str) -> Result<bool, SqlxError>;
    async fn revoke_token<'a>( // Added lifetime 'a
        &self,
        jti: &'a str, // Added lifetime 'a
        user_id: Uuid,
        token_type: TokenType,
        expires_at: DateTime<Utc>,
        reason: Option<&'a str>, // Added lifetime 'a
    ) -> Result<(), SqlxError>;
    async fn revoke_all_user_tokens<'a>( // Added lifetime 'a
        &self,
        user_id: Uuid,
        reason: Option<&'a str>, // Added lifetime 'a
    ) -> Result<u64, SqlxError>;
    async fn cleanup_expired_tokens(&self) -> Result<u64, SqlxError>;
}


/// Service for managing token revocation
pub struct TokenRevocationService {
    pool: PgPool,
    // Changed to use the trait for active_token_service
    active_token_service: Option<Arc<dyn ActiveTokenServiceTrait>>,
}

impl TokenRevocationService {
    /// Create a new TokenRevocationService
    pub fn new(pool: PgPool) -> Self {
        Self { pool, active_token_service: None }
    }
    
    /// Set the active token service
    pub fn set_active_token_service(&mut self, service: Arc<dyn ActiveTokenServiceTrait>) {
        self.active_token_service = Some(service);
    }
}

#[async_trait]
impl TokenRevocationServiceTrait for TokenRevocationService {
    /// Check if a token is revoked
    // pub removed from trait impl methods
    async fn is_token_revoked(&self, jti: &str) -> Result<bool, SqlxError> {
        debug!("Checking if token is revoked: {}", jti);
        
        let result = sqlx::query!(
            r#"
            SELECT EXISTS(SELECT 1 FROM revoked_tokens WHERE jti = $1) as "exists!"
            "#,
            jti
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.exists)
    }

    /// Revoke a token
    async fn revoke_token<'a>( // Removed pub, added lifetime 'a
        &self,
        jti: &'a str, // Added lifetime 'a
        user_id: Uuid,
        token_type: TokenType,
        expires_at: DateTime<Utc>,
        reason: Option<&'a str>, // Added lifetime 'a
    ) -> Result<(), SqlxError> {
        info!("Revoking token for user {}: {}", user_id, jti);
        
        let token_type_str = match token_type {
            TokenType::Access => "access",
            TokenType::Refresh => "refresh",
        };

        sqlx::query!(
            r#"
            INSERT INTO revoked_tokens (id, jti, user_id, token_type, expires_at, revoked_at, reason)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
            Uuid::new_v4(),
            jti,
            user_id,
            token_type_str,
            expires_at,
            Utc::now(),
            reason,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Revoke all tokens for a user
    async fn revoke_all_user_tokens<'a>( // Removed pub, added lifetime 'a
        &self,
        user_id: Uuid,
        reason: Option<&'a str>, // Added lifetime 'a
    ) -> Result<u64, SqlxError> {
        warn!("Revoking all tokens for user: {}", user_id);
        
        let mut revoked_count = 0;
        
        // Get all active tokens for the user if the active token service is available
        if let Some(active_service) = &self.active_token_service {
            match active_service.get_user_tokens(user_id).await {
                Ok(tokens) => {
                    info!("Found {} active tokens for user {}", tokens.len(), user_id);
                    
                    // Revoke each token
                    for token in tokens {
                        let token_type = match token.token_type.as_str() {
                            "access" => TokenType::Access,
                            "refresh" => TokenType::Refresh,
                            _ => continue, // Skip invalid token types
                        };
                        
                        match self.revoke_token(
                            &token.jti,
                            user_id,
                            token_type,
                            token.expires_at,
                            reason,
                        ).await {
                            Ok(_) => {
                                revoked_count += 1;
                                debug!("Revoked token {} for user {}", token.jti, user_id);
                            },
                            Err(e) => {
                                error!("Failed to revoke token {} for user {}: {:?}", token.jti, user_id, e);
                            }
                        }
                    }
                    
                    // Remove all active tokens for the user
                    if let Err(e) = active_service.remove_all_user_tokens(user_id).await {
                        error!("Failed to remove active tokens for user {user_id}: {e:?}");
                    }
                },
                Err(e) => {
                    error!("Failed to get active tokens for user {user_id}: {e:?}");
                }
            }
        } else {
            warn!("Active token service not available, cannot revoke all tokens for user {user_id}");
        }
        
        Ok(revoked_count)
    }

    /// Clean up expired revoked tokens
    async fn cleanup_expired_tokens(&self) -> Result<u64, SqlxError> { // Removed pub
        debug!("Cleaning up expired revoked tokens");
        
        let result = sqlx::query!(
            r#"
            DELETE FROM revoked_tokens
            WHERE expires_at < $1
            "#,
            Utc::now(),
        )
        .execute(&self.pool)
        .await?;

        info!("Cleaned up {} expired revoked tokens", result.rows_affected());
        
        Ok(result.rows_affected())
    }
}
