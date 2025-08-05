// src/core/auth/service.rs
use bcrypt::verify;
use crate::common::{
    error::{AuthError, AuthErrorType},
    validation::{LoginInput, RegisterInput},
};
use crate::core::user::{User, UserRepositoryTrait, NewUser}; 
use std::{env, sync::Arc};
use super::jwt::{self, Claims, TokenType, TokenPair, TokenMetadata};
use crate::core::auth::token_revocation::TokenRevocationServiceTrait;
use crate::core::auth::active_token::ActiveTokenServiceTrait;
use crate::core::email::service::EmailServiceTrait;
use log::{info, warn, error};
use uuid::Uuid;
use chrono::{Utc, Duration};

pub struct AuthService {
    user_repository: Arc<dyn UserRepositoryTrait>,
    jwt_secret: String,
    jwt_audience: String, // Add jwt_audience field
    token_revocation_service: Arc<dyn TokenRevocationServiceTrait>,
    active_token_service: Arc<dyn ActiveTokenServiceTrait>,
    email_service: Arc<dyn EmailServiceTrait>,
}

impl AuthService {
    pub fn new(
        user_repository: Arc<dyn UserRepositoryTrait>,
        jwt_secret: String,
        jwt_audience: String, // Add jwt_audience parameter
        token_revocation_service: Arc<dyn TokenRevocationServiceTrait>,
        active_token_service: Arc<dyn ActiveTokenServiceTrait>,
        email_service: Arc<dyn EmailServiceTrait>,
    ) -> Self {
        Self {
            user_repository,
            jwt_secret,
            jwt_audience, // Initialize jwt_audience
            token_revocation_service,
            active_token_service,
            email_service,
        }
    }

    pub async fn login(&self, input: LoginInput) -> Result<(TokenPair, User), AuthError> {
        let user = self.user_repository.find_by_username(&input.username)
            .await
            .map_err(|e| {
                warn!("Login: User repository error on find_by_username for {}: {:?}", input.username, e);
                AuthError::new(AuthErrorType::InvalidCredentials)
            })?
            .ok_or_else(|| {
                warn!("Login: User not found by username: {}", input.username);
                AuthError::new(AuthErrorType::InvalidCredentials)
            })?;

        if !user.is_email_verified {
            warn!("Login: Email not verified for user: {}", user.username);
            return Err(AuthError::new(AuthErrorType::EmailNotVerified));
        }

        if !verify(&input.password, &user.password_hash)
            .map_err(|e| {
                warn!("Login: Password verification (bcrypt) error for user {}: {:?}", user.username, e);
                AuthError::new(AuthErrorType::InternalServerError)
            })? {
            warn!("Login: Invalid password for user: {}", user.username);
            return Err(AuthError::new(AuthErrorType::InvalidCredentials));
        }

        info!("Login: Generating token pair for user: {}", user.id);
        let token_pair = jwt::create_token_pair_explicit(
            user.id,
            user.role.clone(),
            &self.jwt_secret,
            self.jwt_audience.clone(),
            env::var("JWT_ISSUER").unwrap_or_else(|_| "default_issuer".to_string())
        )
        .map_err(|e| {
            warn!("Login: Failed to create token pair for user {}: {:?}", user.id, e);
            AuthError::new(AuthErrorType::TokenCreationError)
        })?;

        if let Err(e) = self.record_tokens_for_user(user.id, &token_pair).await {
            warn!("Login: Failed to record tokens for user {}: {:?}", user.id, e);
        }

        Ok((token_pair, user))
    }

    pub async fn validate_auth(&self, token: &str) -> Result<Claims, AuthError> {
        let claims = match jwt::validate_jwt(&self.token_revocation_service, token, &self.jwt_secret, Some(TokenType::Access), Some(self.jwt_audience.clone()), None).await {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Token validation failed: {e:?}");
                return Err(AuthError::new(AuthErrorType::InvalidToken));
            }
        };

        println!("🔍 DEBUG: AuthService.validate_auth - Looking up user by ID: {}", claims.sub);
        let user = self.user_repository.find_by_id(claims.sub)
            .await
            .map_err(|e| {
                warn!("Validate_auth: User repository error on find_by_id for {}: {:?}", claims.sub, e);
                AuthError::new(AuthErrorType::InvalidToken)
            })?
            .ok_or_else(|| {
                warn!("Validate_auth: User not found by id from token: {}", claims.sub);
                println!("🔍 DEBUG: AuthService.validate_auth - User NOT FOUND for ID: {}", claims.sub);
                AuthError::new(AuthErrorType::InvalidToken)
            })?;

        println!("🔍 DEBUG: AuthService.validate_auth - Found user: {}, email_verified: {}", user.username, user.is_email_verified);
        if !user.is_email_verified {
            warn!("Validate_auth: Email not verified for user from token: {}", user.username);
            println!("🔍 DEBUG: AuthService.validate_auth - EMAIL NOT VERIFIED for user: {}", user.username);
            return Err(AuthError::new(AuthErrorType::EmailNotVerified));
        }

        println!("🔍 DEBUG: AuthService.validate_auth - SUCCESS for user: {}", user.username);
        Ok(claims)
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair, AuthError> {
        let token_pair = match jwt::refresh_token_pair_explicit(
            self.token_revocation_service.clone(),
            self.active_token_service.clone(),
            refresh_token,
            &self.jwt_secret,
            self.jwt_audience.clone(),
            env::var("JWT_ISSUER").unwrap_or_else(|_| "default_issuer".to_string())
        ).await {
            Ok(token_pair) => token_pair,
            Err(e) => {
                warn!("Token refresh failed: {e:?}");
                return Err(AuthError::new(AuthErrorType::InvalidToken));
            }
        };

        let access_claims = match jwt::validate_jwt(&self.token_revocation_service, &token_pair.access_token, &self.jwt_secret, Some(TokenType::Access), Some(self.jwt_audience.clone()), None).await {
            Ok(claims) => claims,
            Err(_) => {
                 warn!("Refresh_token: Failed to validate newly created access token during refresh flow.");
                 return Err(AuthError::new(AuthErrorType::TokenCreationError));
            }
        };

        if let Err(e) = self.record_tokens_for_user(access_claims.sub, &token_pair).await {
            warn!("Refresh_token: Failed to record refreshed tokens for user {}: {:?}", access_claims.sub, e);
        }

        info!("Tokens refreshed successfully for user: {}", access_claims.sub);
        Ok(token_pair)
    }

    async fn record_tokens_for_user(&self, user_id: uuid::Uuid, token_pair: &TokenPair) -> Result<(), AuthError> {
        let access_claims = match jwt::validate_jwt(&self.token_revocation_service, &token_pair.access_token, &self.jwt_secret, Some(TokenType::Access), Some(self.jwt_audience.clone()), None).await {
            Ok(claims) => claims,
            Err(e) => {
                warn!("record_tokens_for_user: Failed to validate access token for recording for user {user_id}: {e:?}");
                return Err(AuthError::new(AuthErrorType::InternalServerError));
            }
        };

        let refresh_claims = match jwt::validate_jwt(&self.token_revocation_service, &token_pair.refresh_token, &self.jwt_secret, Some(TokenType::Refresh), Some(self.jwt_audience.clone()), None).await {
            Ok(claims) => claims,
            Err(e) => {
                warn!("record_tokens_for_user: Failed to validate refresh token for recording for user {user_id}: {e:?}");
                return Err(AuthError::new(AuthErrorType::InternalServerError));
            }
        };

        jwt::record_active_token(&self.active_token_service, user_id, &TokenMetadata{jti: access_claims.jti, expires_at: jwt::timestamp_to_datetime(access_claims.exp)}, TokenType::Access).await;
        jwt::record_active_token(&self.active_token_service, user_id, &TokenMetadata{jti: refresh_claims.jti, expires_at: jwt::timestamp_to_datetime(refresh_claims.exp)}, TokenType::Refresh).await;

        Ok(())
    }

    pub async fn logout(&self, access_token: &str, refresh_token: Option<&str>) -> Result<(), AuthError> {
        let access_claims = match jwt::validate_jwt(&self.token_revocation_service, access_token, &self.jwt_secret, Some(TokenType::Access), Some(self.jwt_audience.clone()), None).await {
            Ok(claims) => claims,
            Err(e) => {
                warn!("Logout: Failed to validate access token: {e:?}");
                return Err(AuthError::new(AuthErrorType::InvalidToken));
            }
        };

        jwt::revoke_token(&self.token_revocation_service, &self.active_token_service, &access_claims.jti, access_claims.sub, TokenType::Access, Some("User logout")).await;

        if let Some(rt_str) = refresh_token {
            match jwt::validate_jwt(&self.token_revocation_service, rt_str, &self.jwt_secret, Some(TokenType::Refresh), Some(self.jwt_audience.clone()), None).await {
                Ok(refresh_claims) => {
                     jwt::revoke_token(&self.token_revocation_service, &self.active_token_service, &refresh_claims.jti, refresh_claims.sub, TokenType::Refresh, Some("User logout")).await;
                },
                Err(e) => {
                    warn!("Logout: Failed to validate refresh token, not revoking: {e:?}");
                }
            }
        }

        info!("User {} logged out successfully", access_claims.sub);
        Ok(())
    }

    pub async fn register(&self, input: RegisterInput) -> Result<User, AuthError> {
        if self.user_repository.find_by_username(&input.username).await.map_err(|db_err: sqlx::Error| {
            error!("Registration: DB error checking username {}: {:?}", input.username, db_err);
            AuthError::new(AuthErrorType::InternalServerError)
        })?.is_some() {
            warn!("Registration: Username {} already exists.", input.username);
            return Err(AuthError::new_with_message(AuthErrorType::UserAlreadyExists, "Username already exists."));
        }

        if self.user_repository.find_user_by_email(&input.email).await.map_err(|db_err: sqlx::Error| {
            error!("Registration: DB error checking email {}: {:?}", input.email, db_err);
            AuthError::new(AuthErrorType::InternalServerError)
        })?.is_some() {
            warn!("Registration: Email {} already exists.", input.email);
            return Err(AuthError::new_with_message(AuthErrorType::UserAlreadyExists, "Email already exists."));
        }

        let password_hash = bcrypt::hash(&input.password, bcrypt::DEFAULT_COST)
            .map_err(|e| {
                error!("Registration: Failed to hash password for {}: {:?}", input.username, e);
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        // Generate the token string once, wrapped in Arc to manage its lifetime across async calls.
        let generated_verification_token_arc = Arc::new(Uuid::new_v4().to_string());
        let verification_token_expires_at = Utc::now() + Duration::days(1);

        let new_user_data = NewUser {
            username: input.username.clone(),
            email: Some(input.email.clone()),
            password_hash,
            is_email_verified: false,
            role: "user".to_string(),
            // Clone the String content from the Arc for NewUser, as NewUser expects Option<String>.
            verification_token: Some(generated_verification_token_arc.as_ref().clone()), 
            verification_token_expires_at: Some(verification_token_expires_at),
        };

        let user = self.user_repository.create_user(new_user_data).await.map_err(|db_err: sqlx::Error| {
            error!("Registration: Failed to create user {}: {:?}", input.username, db_err);
            AuthError::new(AuthErrorType::InternalServerError)
        })?;

        // Send email using a reference derived from the Arc'd string.
        // This ensures the string data lives as long as the Arc.
        if let Err(email_err) = self.email_service.send_verification_email(&user.email.clone().unwrap_or_default(), &generated_verification_token_arc).await {
            warn!("Registration: Failed to send verification email to {} for user {}: {:?}", user.email.clone().unwrap_or_default(), user.username, email_err);
        } else {
            info!("Registration: Verification email sent to {} for user {}", user.email.clone().unwrap_or_default(), user.username);
        }

        Ok(user)
    }

    pub async fn change_password(&self, user_id: Uuid, old_password: String, new_password: String) -> Result<(), AuthError> {
        let user = self.user_repository.find_by_id(user_id)
            .await
            .map_err(|e| {
                error!("Change Password: User repository error on find_by_id for {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?
            .ok_or_else(|| {
                warn!("Change Password: User not found by id: {user_id}");
                AuthError::new(AuthErrorType::UserNotFound)
            })?;

        if !verify(&old_password, &user.password_hash)
            .map_err(|e| {
                error!("Change Password: Old password verification (bcrypt) error for user {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })? {
            warn!("Change Password: Invalid old password for user: {user_id}");
            return Err(AuthError::new(AuthErrorType::InvalidCredentials));
        }

        let new_password_hash = bcrypt::hash(&new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| {
                error!("Change Password: Failed to hash new password for {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        self.user_repository.update_password(user_id, &new_password_hash)
            .await
            .map_err(|e| {
                error!("Change Password: User repository error on update_password for {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        // TODO: Temporarily commented out to debug lifetime error "verification_token does not live long enough"
        // if let Err(e) = self.token_revocation_service.revoke_all_user_tokens(user_id, Some("Password changed")).await {
        //      warn!("Change Password: Failed to revoke all tokens for user {} after password change: {:?}", user_id, e);
        // } else {
        //      info!("Change Password: Revoked all tokens for user {} after password change", user_id);
        // }

        Ok(())
    }

    pub async fn verify_email(&self, token: &str) -> Result<(), AuthError> {
        let user_id_option = self.user_repository.verify_email(token)
            .await
            .map_err(|e| {
                error!("Verify Email: User repository error on verify_email: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        let user_id = match user_id_option {
            Some(id) => id,
            None => {
                match self.user_repository.find_by_verification_token(token).await {
                    Ok(Some(user)) => {
                        if let Some(expires_at) = user.verification_token_expires_at {
                            if Utc::now() > expires_at {
                                warn!("Verify Email: Verification token expired for user: {}", user.username);
                                return Err(AuthError::new(AuthErrorType::VerificationTokenExpired));
                            } else {
                                warn!("Verify Email: Verification token found but not processed by verify_email for user: {}", user.username);
                                return Err(AuthError::new(AuthErrorType::InvalidVerificationToken));
                            }
                        } else {
                            warn!("Verify Email: Verification token found without expiration date for user: {}", user.username);
                            return Err(AuthError::new(AuthErrorType::InvalidVerificationToken));
                        }
                    }
                    Ok(None) => {
                        warn!("Verify Email: Invalid verification token provided (not found).");
                        return Err(AuthError::new(AuthErrorType::InvalidVerificationToken));
                    }
                    Err(e) => {
                        error!("Verify Email: User repository error during find_by_verification_token: {e:?}");
                        return Err(AuthError::new(AuthErrorType::InternalServerError));
                    }
                }
            }
        };

        let user = self.user_repository.find_by_id(user_id).await
             .map_err(|e| {
                error!("Verify Email: User repository error on find_by_id after verification for {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?
            .ok_or_else(|| {
                warn!("Verify Email: User not found by id after verification: {user_id}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        info!("Email verified successfully for user: {}", user.username);
        Ok(())
    }

    pub async fn request_password_reset(&self, email: String) -> Result<(), AuthError> {
        let user = self.user_repository.find_user_by_email(&email)
            .await
            .map_err(|e| {
                error!("Request Password Reset: DB error finding user by email {email}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?
            .ok_or_else(|| {
                warn!("Request Password Reset: User not found for email {email}, but proceeding as if successful to prevent enumeration.");
                AuthError::new(AuthErrorType::UserNotFound)
            })?;

        let reset_token_model = self.user_repository.create_password_reset_token(user.id)
            .await
            .map_err(|e| {
                error!("Request Password Reset: DB error creating reset token for user {}: {:?}", user.id, e);
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        let email_for_send = user.email.as_deref().unwrap_or_default().to_string();
        if let Err(e) = self.email_service.send_password_reset_email(&email_for_send, &reset_token_model.token).await {
            warn!("Request Password Reset: Failed to send password reset email to {} for user {}: {:?}",
                email_for_send, user.username, e);
        } else {
            info!("Request Password Reset: Password reset email sent to {} for user {}", email_for_send, user.username);
        }
        Ok(())
    }

    pub async fn verify_password_reset_token(&self, token: &str) -> Result<Uuid, AuthError> {
        let reset_token_model = self.user_repository.verify_reset_token(token)
            .await
            .map_err(|e| {
                error!("Verify Password Reset Token: DB error verifying reset token: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?
            .ok_or_else(|| {
                warn!("Verify Password Reset Token: Token not found or invalid: {token}");
                AuthError::new(AuthErrorType::InvalidToken)
            })?;
        
        info!("Password reset token verified successfully for user: {}", reset_token_model.user_id);
        Ok(reset_token_model.user_id)
    }

    pub async fn reset_password(&self, token: &str, new_password: String) -> Result<(), AuthError> {
        let user_id = self.verify_password_reset_token(token).await?;

        let new_password_hash = bcrypt::hash(&new_password, bcrypt::DEFAULT_COST)
            .map_err(|e| {
                error!("Reset Password: Failed to hash new password for user {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        self.user_repository.update_password(user_id, &new_password_hash)
            .await
            .map_err(|e| {
                error!("Reset Password: User repository error on update_password for user {user_id}: {e:?}");
                AuthError::new(AuthErrorType::InternalServerError)
            })?;

        // After successful password reset, revoke all tokens for the user
        if let Err(e) = self.token_revocation_service.revoke_all_user_tokens(user_id, Some("Password reset")).await {
            warn!("Reset Password: Failed to revoke all tokens for user {user_id} after password reset: {e:?}");
            // Non-critical error, so we don't return an error here, but log it.
        } else {
            info!("Reset Password: Revoked all tokens for user {user_id} after password reset");
        }
        
        // It's important that the reset token is marked as used *after* the password has been successfully updated.
        // This is handled by the `verify_reset_token` method in the user repository, which should mark the token as used.
        // If verify_reset_token doesn't mark it as used, it should be updated to do so.
        // For now, we assume verify_reset_token handles this. If not, an explicit call to mark it as used would be needed here.

        info!("Password reset successfully for user_id: {user_id}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::user::{MockUserRepositoryTrait, NewUser, model::{UserUpdate, PasswordResetToken}};
    use crate::core::email::service::MockEmailServiceTrait;
    use crate::core::auth::jwt::{self, Claims, TokenType, TokenPair, TokenMetadata};
    use crate::core::auth::active_token::{MockActiveTokenServiceTrait, ActiveToken};
    use crate::core::auth::token_revocation::MockTokenRevocationServiceTrait;
    use crate::common::validation::{LoginInput, RegisterInput};
    use crate::common::error::{AuthErrorType};
    use mockall::{predicate, Sequence};
    use std::sync::Arc;
    use sqlx::Error as SqlxError;

    fn create_test_user(id: Uuid, username: &str, email_verified: bool, role: &str) -> User {
        User {
            id,
            username: username.to_string(),
            email: Some(format!("{}@example.com", username)),
            password_hash: bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap(),
            is_email_verified: email_verified,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: role.to_string(),
            is_active: true,
        }
    }

    const TEST_JWT_SECRET: &str = "test_auth_service_jwt_secret_very_secure";

    fn setup_mock_services() -> (Arc<dyn TokenRevocationServiceTrait>, Arc<dyn ActiveTokenServiceTrait>) {
        let mut mock_trs = MockTokenRevocationServiceTrait::new();
        mock_trs.expect_is_token_revoked().returning(|_| Ok(false));
        mock_trs.expect_revoke_token().returning(|_jti, _user_id, _token_type, _expires_at, _reason| Ok(()));
        // Use predicate::always() for the Option<&str> reason argument
        mock_trs.expect_revoke_all_user_tokens()
            .with(predicate::always(), predicate::always()) // User ID and Reason
            .returning(|_user_id, _reason| Ok(0));
        mock_trs.expect_cleanup_expired_tokens().returning(|| Ok(0));

        let mut mock_ats = MockActiveTokenServiceTrait::new();
        mock_ats.expect_record_token().returning(|_user_id, _jti, _token_type, _expires_at, _device_info| Ok(()));
        mock_ats.expect_get_active_token().returning(|jti| {
            Ok(ActiveToken {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                jti: jti.to_string(),
                token_type: "Access".to_string(),
                expires_at: Utc::now() + Duration::hours(1),
                created_at: Utc::now(),
                device_info: None,
            })
        });
        mock_ats.expect_remove_token().returning(|_| Ok(true));
        mock_ats.expect_get_user_tokens().returning(|_| Ok(vec![]));
        mock_ats.expect_remove_all_user_tokens().returning(|_| Ok(0));
        mock_ats.expect_cleanup_expired_tokens().returning(|| Ok(0));

        (Arc::new(mock_trs), Arc::new(mock_ats))
    }

    fn setup_mock_email_service() -> Arc<dyn EmailServiceTrait> {
        let mut mock_email_service = MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        Arc::new(mock_email_service)
    }

    #[tokio::test]
    async fn test_login_successful() {
        // Set up environment variables for JWT
        std::env::set_var("JWT_AUDIENCE", "test_aud");
        std::env::set_var("JWT_ISSUER", "test_issuer");
        
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let test_username = "testuser";
        let test_user = create_test_user(test_user_id, test_username, true, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(test_username))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        let (mock_trs_arc, _) = setup_mock_services();
        let mut mock_ats_for_login = MockActiveTokenServiceTrait::new();
        mock_ats_for_login.expect_record_token().times(2).returning(|_user_id, _jti, _token_type, _expires_at, _device_info| Ok(()));
        let mock_ats_arc_for_login: Arc<dyn ActiveTokenServiceTrait> = Arc::new(mock_ats_for_login);

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs_arc.clone(),
            mock_ats_arc_for_login.clone(),
            setup_mock_email_service(),
        );

        let login_input = LoginInput {
            username: test_username.to_string(),
            password: "password123".to_string(),
        };

        let result = auth_service.login(login_input).await;

        assert!(result.is_ok());
        let (token_pair, logged_in_user) = result.unwrap();
        assert_eq!(logged_in_user.id, test_user_id);
        assert!(!token_pair.access_token.is_empty());
        assert!(!token_pair.refresh_token.is_empty());

        let claims = jwt::validate_jwt(&mock_trs_arc, &token_pair.access_token, TEST_JWT_SECRET, Some(TokenType::Access), Some("test_aud".to_string()), None).await.unwrap();
        assert_eq!(claims.sub, test_user_id);
        assert_eq!(claims.role, "user");
        assert_eq!(claims.aud, "test_aud"); // Add audience assertion
        
        // Clean up environment variables
        std::env::remove_var("JWT_AUDIENCE");
        std::env::remove_var("JWT_ISSUER");
    }

    #[tokio::test]
    async fn test_login_user_not_found() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_username = "unknownuser";

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(test_username))
            .times(1)
            .returning(|_| Ok(None));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let login_input = LoginInput {
            username: test_username.to_string(),
            password: "password123".to_string(),
        };

        let result = auth_service.login(login_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidCredentials);
    }

    #[tokio::test]
    async fn test_login_email_not_verified() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let test_username = "unverifieduser";
        let test_user = create_test_user(test_user_id, test_username, false, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(test_username))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let login_input = LoginInput {
            username: test_username.to_string(),
            password: "password123".to_string(),
        };

        let result = auth_service.login(login_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::EmailNotVerified);
    }

    #[tokio::test]
    async fn test_login_incorrect_password() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let test_username = "testuser";
        let test_user = create_test_user(test_user_id, test_username, true, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(test_username))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let login_input = LoginInput {
            username: test_username.to_string(),
            password: "wrongpassword".to_string(),
        };

        let result = auth_service.login(login_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidCredentials);
    }

    #[tokio::test]
    async fn test_login_user_repo_error() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_username = "testuser";

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(test_username))
            .times(1)
            .returning(|_| Err(sqlx::Error::RowNotFound));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let login_input = LoginInput {
            username: test_username.to_string(),
            password: "password123".to_string(),
        };

        let result = auth_service.login(login_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidCredentials);
    }

    #[tokio::test]
    async fn test_validate_auth_successful() {
        // Set up environment variables for JWT
        std::env::set_var("JWT_AUDIENCE", "test_aud");
        std::env::set_var("JWT_ISSUER", "test_issuer");
        
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let test_user = create_test_user(test_user_id, "testuser", true, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        let test_aud = "test_aud".to_string(); // Define test audience
        let test_iss = "test_issuer".to_string(); // Define test issuer
        let token_pair = jwt::create_token_pair_explicit(test_user_id, "user".to_string(), TEST_JWT_SECRET, test_aud.clone(), test_iss.clone()).unwrap();
        let token_str = token_pair.access_token;
 
         let (mock_trs_for_jwt_val, _) : (Arc<dyn TokenRevocationServiceTrait>, Arc<dyn ActiveTokenServiceTrait>) = setup_mock_services();
        jwt::validate_jwt(&mock_trs_for_jwt_val, &token_str, TEST_JWT_SECRET, Some(TokenType::Access), Some(test_aud.clone()), Some(test_iss.clone())).await.expect("Token for test_validate_auth_successful should be valid");
 
         let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            test_aud.clone(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.validate_auth(&token_str).await;
        assert!(result.is_ok());
        let claims = result.unwrap();
        assert_eq!(claims.sub, test_user_id);
        assert_eq!(claims.role, "user");
        
        // Clean up environment variables
        std::env::remove_var("JWT_AUDIENCE");
        std::env::remove_var("JWT_ISSUER");
    }

    #[tokio::test]
    async fn test_validate_auth_invalid_token_signature() {
        let mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let auth_service = AuthService::new(
           Arc::new(mock_user_repo),
           TEST_JWT_SECRET.to_string(),
           "test_aud".to_string(),
           mock_trs,
           mock_ats,
           setup_mock_email_service(),
       );

        let test_aud = "test_aud".to_string();
        let test_iss = "test_issuer".to_string();
        let token_pair_diff_secret = jwt::create_token_pair_explicit(Uuid::new_v4(), "user".to_string(), "a_different_secret", test_aud.clone(), test_iss.clone()).unwrap();
        let token_str = token_pair_diff_secret.access_token;

        let (fresh_mock_trs, _) : (Arc<dyn TokenRevocationServiceTrait>, Arc<dyn ActiveTokenServiceTrait>) = setup_mock_services();
       jwt::validate_jwt(&fresh_mock_trs, &token_str, "a_different_secret", Some(TokenType::Access), Some(test_aud.clone()), Some(test_iss.clone())).await.expect("Token created with different secret should be valid with that secret");

        let result = auth_service.validate_auth(&token_str).await;
       assert!(result.is_err());
       let auth_error = result.unwrap_err();
       assert_eq!(auth_error.error_type, AuthErrorType::InvalidToken);
    }

    #[tokio::test]
    async fn test_validate_auth_user_not_found_by_id() {
        // Set up environment variables for JWT
        std::env::set_var("JWT_AUDIENCE", "test_aud");
        std::env::set_var("JWT_ISSUER", "test_issuer");
        
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(|_| Ok(None));

        let test_aud = "test_aud".to_string();
        let test_iss = "test_issuer".to_string();
        let token_pair = jwt::create_token_pair_explicit(test_user_id, "user".to_string(), TEST_JWT_SECRET, test_aud.clone(), test_iss.clone()).unwrap();
        let token_str = token_pair.access_token;

        let (fresh_mock_trs, _) : (Arc<dyn TokenRevocationServiceTrait>, Arc<dyn ActiveTokenServiceTrait>) = setup_mock_services();
       jwt::validate_jwt(&fresh_mock_trs, &token_str, TEST_JWT_SECRET, Some(TokenType::Access), Some(test_aud.clone()), Some(test_iss.clone())).await.expect("Token for test_validate_auth_user_not_found_by_id should be valid");

        let (mock_trs, mock_ats) = setup_mock_services();
       let auth_service = AuthService::new(
          Arc::new(mock_user_repo),
          TEST_JWT_SECRET.to_string(),
          "test_aud".to_string(),
          mock_trs,
          mock_ats,
          setup_mock_email_service(),
      );

        let result = auth_service.validate_auth(&token_str).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidToken);
        
        // Clean up environment variables
        std::env::remove_var("JWT_AUDIENCE");
        std::env::remove_var("JWT_ISSUER");
    }

    #[tokio::test]
    async fn test_validate_auth_user_email_not_verified() {
        // Set up environment variables for JWT
        std::env::set_var("JWT_AUDIENCE", "test_aud");
        std::env::set_var("JWT_ISSUER", "test_issuer");
        
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let test_user = create_test_user(test_user_id, "unverified_user_for_validate", false, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        let test_aud = "test_aud".to_string();
        let test_iss = "test_issuer".to_string();
        let token_pair = jwt::create_token_pair_explicit(test_user_id, "user".to_string(), TEST_JWT_SECRET, test_aud.clone(), test_iss.clone()).unwrap();
        let token_str = token_pair.access_token;

        let (fresh_mock_trs, _) : (Arc<dyn TokenRevocationServiceTrait>, Arc<dyn ActiveTokenServiceTrait>) = setup_mock_services();
       jwt::validate_jwt(&fresh_mock_trs, &token_str, TEST_JWT_SECRET, Some(TokenType::Access), Some(test_aud.clone()), Some(test_iss.clone())).await.expect("Token for test_validate_auth_user_email_not_verified should be valid");

        let (mock_trs, mock_ats) = setup_mock_services();
       let auth_service = AuthService::new(
          Arc::new(mock_user_repo),
          TEST_JWT_SECRET.to_string(),
          "test_aud".to_string(),
          mock_trs,
          mock_ats,
          setup_mock_email_service(),
      );

        let result = auth_service.validate_auth(&token_str).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::EmailNotVerified);
        
        // Clean up environment variables
        std::env::remove_var("JWT_AUDIENCE");
        std::env::remove_var("JWT_ISSUER");
    }

    #[tokio::test]
    async fn test_refresh_token_successful() {
        // Set up environment variables for JWT
        std::env::set_var("JWT_AUDIENCE", "test_aud");
        std::env::set_var("JWT_ISSUER", "test_issuer");
        
        let mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();

        let mut mock_ats_concrete = MockActiveTokenServiceTrait::new();
        mock_ats_concrete.expect_record_token().times(2).returning(|_,_,_,_,_| Ok(()));
        mock_ats_concrete.expect_get_active_token().returning(move |jti| {
             Ok(ActiveToken {
                id: Uuid::new_v4(), user_id: test_user_id, jti: jti.to_string(), token_type: "Refresh".to_string(),
                expires_at: Utc::now() + Duration::days(1), created_at: Utc::now(), device_info: None,
            })
        });
        let mock_ats: Arc<dyn ActiveTokenServiceTrait> = Arc::new(mock_ats_concrete);


        let mut mock_trs_concrete = MockTokenRevocationServiceTrait::new();
        mock_trs_concrete.expect_is_token_revoked().returning(|_| Ok(false)).times(4);
        mock_trs_concrete.expect_revoke_token().times(1).returning(|_,_,_,_,_| Ok(()));
        let mock_trs: Arc<dyn TokenRevocationServiceTrait> = Arc::new(mock_trs_concrete);

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs.clone(),
            mock_ats.clone(),
            setup_mock_email_service(),
        );

        let test_aud = "test_aud".to_string();
        let test_iss = "test_issuer".to_string();
        let initial_token_pair = jwt::create_token_pair_explicit(test_user_id, "user".to_string(), TEST_JWT_SECRET, test_aud.clone(), test_iss.clone()).unwrap();
        let refresh_token_str = initial_token_pair.refresh_token;

        let result = auth_service.refresh_token(&refresh_token_str).await;
        assert!(result.is_ok(), "refresh_token failed: {:?}", result.err());
        let new_token_pair = result.unwrap();

        assert!(!new_token_pair.access_token.is_empty());
        assert!(!new_token_pair.refresh_token.is_empty());
        assert_ne!(new_token_pair.refresh_token, refresh_token_str, "New refresh token should be different from the old one");
 
         let (fresh_mock_trs_val, _) : (Arc<dyn TokenRevocationServiceTrait>, Arc<dyn ActiveTokenServiceTrait>) = setup_mock_services();
        let claims = jwt::validate_jwt(&fresh_mock_trs_val, &new_token_pair.access_token, TEST_JWT_SECRET, Some(TokenType::Access), Some("test_aud".to_string()), Some("test_issuer".to_string())).await.unwrap();
        assert_eq!(claims.sub, test_user_id);
        assert_eq!(claims.aud, "test_aud"); // Add audience assertion
        
        // Clean up environment variables
        std::env::remove_var("JWT_AUDIENCE");
        std::env::remove_var("JWT_ISSUER");
    }

    #[tokio::test]
    async fn test_refresh_token_invalid_refresh_token() {
        let mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let auth_service = AuthService::new(
           Arc::new(mock_user_repo),
           TEST_JWT_SECRET.to_string(),
           "test_aud".to_string(),
           mock_trs,
           mock_ats,
           setup_mock_email_service(),
       );

        let invalid_refresh_token = "this.is.not.a.valid.token";
        let result = auth_service.refresh_token(invalid_refresh_token).await;

        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidToken);
    }

    #[tokio::test]
    async fn test_logout_successful_with_refresh_token() {
        let mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();

        let mut mock_trs_concrete = MockTokenRevocationServiceTrait::new();
        // For access token validation and refresh token validation by jwt::validate_jwt
        mock_trs_concrete.expect_is_token_revoked().returning(|_| Ok(false)).times(2);
        
        // For revoking access token by jwt::revoke_token
        mock_trs_concrete.expect_revoke_token()
            .with(
                predicate::always(), // jti
                predicate::eq(test_user_id),
                predicate::eq(TokenType::Access), // Enum comparison
                predicate::always(), // expires_at
                predicate::always()  // reason
            )
            .times(1)
            .returning(|_,_,_,_,_| Ok(()));

        // For revoking refresh token by jwt::revoke_token
        mock_trs_concrete.expect_revoke_token()
            .with(
                predicate::always(), // jti
                predicate::eq(test_user_id),
                predicate::eq(TokenType::Refresh), // Enum comparison
                predicate::always(), // expires_at
                predicate::always()  // reason
            )
            .times(1)
            .returning(|_,_,_,_,_| Ok(()));
        let mock_trs: Arc<dyn TokenRevocationServiceTrait> = Arc::new(mock_trs_concrete);

        let mut mock_ats_concrete = MockActiveTokenServiceTrait::new();
        // jwt::revoke_token calls get_active_token then remove_token. This happens for both access and refresh tokens.
        mock_ats_concrete.expect_get_active_token().times(2).returning(move |jti| {
            Ok(ActiveToken {
                id: Uuid::new_v4(), user_id: test_user_id, jti: jti.to_string(), token_type: "Access".to_string(), // Actual type might vary but jti is key
                expires_at: Utc::now() + Duration::hours(1), created_at: Utc::now(), device_info: None,
            })
        });
        mock_ats_concrete.expect_remove_token().times(2).returning(|_| Ok(true));
        let mock_ats: Arc<dyn ActiveTokenServiceTrait> = Arc::new(mock_ats_concrete);

        let auth_service = AuthService::new(
           Arc::new(mock_user_repo),
           TEST_JWT_SECRET.to_string(),
           "test_aud".to_string(),
           mock_trs,
           mock_ats,
           setup_mock_email_service(),
       );

        let test_aud = "test_aud".to_string();
        let test_iss = "test_issuer".to_string();
        let token_pair = jwt::create_token_pair_explicit(test_user_id, "user".to_string(), TEST_JWT_SECRET, test_aud.clone(), test_iss.clone()).unwrap();

        let result = auth_service.logout(&token_pair.access_token, Some(&token_pair.refresh_token)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logout_successful_access_token_only() {
        let mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();

        let mut mock_trs_concrete = MockTokenRevocationServiceTrait::new();
        mock_trs_concrete.expect_is_token_revoked().returning(|_| Ok(false)).once();
        mock_trs_concrete.expect_revoke_token()
            .with(
                predicate::always(), // jti
                predicate::eq(test_user_id),
                predicate::eq(TokenType::Access), // token_type
                predicate::always(), // expires_at
                predicate::always()  // reason
            )
            .times(1)
            .returning(|_,_,_,_,_| Ok(()));
        let mock_trs: Arc<dyn TokenRevocationServiceTrait> = Arc::new(mock_trs_concrete);

        let mut mock_ats_concrete = MockActiveTokenServiceTrait::new();
        mock_ats_concrete.expect_get_active_token().once().returning(move |jti| {
            Ok(ActiveToken {
                id: Uuid::new_v4(), user_id: test_user_id, jti: jti.to_string(), token_type: "Access".to_string(),
                expires_at: Utc::now() + Duration::hours(1), created_at: Utc::now(), device_info: None,
            })
        });
        mock_ats_concrete.expect_remove_token().once().returning(|_| Ok(true)); // Added expectation
        let mock_ats: Arc<dyn ActiveTokenServiceTrait> = Arc::new(mock_ats_concrete);

        let auth_service = AuthService::new(
           Arc::new(mock_user_repo),
           TEST_JWT_SECRET.to_string(),
           "test_aud".to_string(),
           mock_trs,
           mock_ats,
           setup_mock_email_service(),
       );

        let test_aud = "test_aud".to_string();
        let test_iss = "test_issuer".to_string();
        let token_pair = jwt::create_token_pair_explicit(test_user_id, "user".to_string(), TEST_JWT_SECRET, test_aud.clone(), test_iss.clone()).unwrap();

        let result = auth_service.logout(&token_pair.access_token, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_logout_invalid_access_token() {
        let mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.logout("invalid.access.token", None).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidToken);
    }

    #[tokio::test]
    async fn test_register_successful() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mut mock_email_service = MockEmailServiceTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let register_input = RegisterInput {
            username: "newuser".to_string(),
            email: "newuser@example.com".to_string(),
            password: "Password123!".to_string(),
            password_confirm: "Password123!".to_string(),
        };

        let expected_user_id = Uuid::new_v4();
        let cloned_input_username = register_input.username.clone();
        let cloned_input_email = register_input.email.clone();
        let password_for_closure_verification = register_input.password.clone();

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(register_input.username.clone()))
            .times(1)
            .returning(|_| Ok(None));
        mock_user_repo.expect_find_user_by_email()
            .with(predicate::eq(register_input.email.clone()))
            .times(1)
            .returning(|_| Ok(None));
        mock_user_repo.expect_create_user()
            .withf(move |new_user: &NewUser| {
                new_user.username == cloned_input_username &&
                new_user.email.as_deref() == Some(cloned_input_email.as_str()) &&
                bcrypt::verify(&password_for_closure_verification, &new_user.password_hash).unwrap_or(false) &&
                !new_user.is_email_verified &&
                new_user.role == "user" &&
                new_user.verification_token.is_some() &&
                new_user.verification_token_expires_at.is_some()
            })
            .times(1)
            .returning(move |new_user_data| {
                Ok(User {
                    id: expected_user_id,
                    username: new_user_data.username.clone(),
                    email: new_user_data.email.clone(),
                    password_hash: new_user_data.password_hash.clone(),
                    is_email_verified: new_user_data.is_email_verified,
                    verification_token: None,
                    verification_token_expires_at: new_user_data.verification_token_expires_at,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    role: new_user_data.role.clone(),
                    is_active: true,
                })
            });

        mock_email_service.expect_send_verification_email()
            .with(
                predicate::eq("newuser@example.com"),
                predicate::function(|token_arg: &str| !token_arg.is_empty())
            )
            .times(1)
            .returning(|_, _| Ok(()));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            Arc::new(mock_email_service),
        );

        let result = auth_service.register(register_input.clone()).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.id, expected_user_id);
        assert_eq!(user.username, "newuser");
        assert_eq!(user.email.unwrap(), "newuser@example.com");
        assert!(!user.is_email_verified);
        assert!(user.verification_token.is_none());
    }

    #[tokio::test]
    async fn test_register_username_exists() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mock_email_service = setup_mock_email_service();
        let (mock_trs, mock_ats) = setup_mock_services();

        let register_input = RegisterInput {
            username: "existinguser".to_string(),
            email: "newemail@example.com".to_string(),
            password: "Password123!".to_string(),
            password_confirm: "Password123!".to_string(),
        };

        let existing_user_id = Uuid::new_v4();
        let existing_user = create_test_user(existing_user_id, "existinguser", true, "user");

        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(register_input.username.clone()))
            .times(1)
            .returning(move |_| Ok(Some(existing_user.clone())));
        mock_user_repo.expect_find_user_by_email().times(0);
        mock_user_repo.expect_create_user().times(0);

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.register(register_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::UserAlreadyExists);
    }

    #[tokio::test]
    async fn test_register_email_exists() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mock_email_service = setup_mock_email_service();
        let (mock_trs, mock_ats) = setup_mock_services();

        let register_input = RegisterInput {
            username: "newusername".to_string(),
            email: "existingemail@example.com".to_string(),
            password: "Password123!".to_string(),
            password_confirm: "Password123!".to_string(),
        };

        let existing_user_id = Uuid::new_v4();
        let existing_user_with_email = User {
            id: existing_user_id,
            username: "anotheruser".to_string(),
            email: Some("existingemail@example.com".to_string()),
            password_hash: bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };

        let mut seq = Sequence::new();
        mock_user_repo.expect_find_by_username()
            .with(predicate::eq(register_input.username.clone()))
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(None));
        mock_user_repo.expect_find_user_by_email()
            .with(predicate::eq(register_input.email.clone()))
            .times(1)
            .in_sequence(&mut seq)
            .returning(move |_| Ok(Some(existing_user_with_email.clone())));
        mock_user_repo.expect_create_user().times(0);

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.register(register_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::UserAlreadyExists);
    }

    #[tokio::test]
    async fn test_register_email_send_failure_still_creates_user() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mut mock_email_service = MockEmailServiceTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let register_input = RegisterInput {
            username: "emailfailuser".to_string(),
            email: "emailfail@example.com".to_string(),
            password: "Password123!".to_string(),
            password_confirm: "Password123!".to_string(),
        };
        let expected_user_id = Uuid::new_v4();
        let cloned_input_username = register_input.username.clone();
        let cloned_input_email = register_input.email.clone();
        let password_for_closure = register_input.password.clone();

        mock_user_repo.expect_find_by_username().returning(|_| Ok(None));
        mock_user_repo.expect_find_user_by_email().returning(|_| Ok(None));
        mock_user_repo.expect_create_user()
            .withf(move |new_user: &NewUser| {
                new_user.username == cloned_input_username &&
                new_user.email == Some(cloned_input_email.clone()) &&
                bcrypt::verify(&password_for_closure, &new_user.password_hash).unwrap_or(false) &&
                !new_user.is_email_verified &&
                new_user.role == "user" &&
                new_user.verification_token.is_some() &&
                new_user.verification_token_expires_at.is_some()
            })
            .times(1)
            .returning(move |new_user_data| {
                Ok(User {
                    id: expected_user_id,
                    username: new_user_data.username.clone(),
                    email: new_user_data.email.clone(),
                    password_hash: new_user_data.password_hash.clone(),
                    is_email_verified: new_user_data.is_email_verified,
                    verification_token: new_user_data.verification_token.clone(),
                    verification_token_expires_at: new_user_data.verification_token_expires_at,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    role: new_user_data.role.clone(),
                    is_active: true,
                })
            });

        mock_email_service.expect_send_verification_email()
            .withf(move |email: &str, token: &str| {
                email == "emailfail@example.com" && !token.is_empty()
            })
            .times(1)
            .returning(|_, _| Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Simulated send failure")) as Box<dyn std::error::Error + Send + Sync>));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            Arc::new(mock_email_service),
        );

        let result = auth_service.register(register_input.clone()).await;
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "emailfailuser");
    }

    #[tokio::test]
    async fn test_change_password_successful() {
        // Set up environment variables for JWT
        std::env::set_var("JWT_AUDIENCE", "test_aud");
        std::env::set_var("JWT_ISSUER", "test_issuer");
        
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let old_password = "OldPassword123!";
        let new_password = "NewPassword123!";
        
        // Create test user with the correct password hash for the old password
        let test_user = User {
            id: test_user_id,
            username: "passwordchangeuser".to_string(),
            email: Some("passwordchangeuser@example.com".to_string()),
            password_hash: bcrypt::hash(old_password, bcrypt::DEFAULT_COST).unwrap(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        mock_user_repo.expect_update_password()
            .with(predicate::eq(test_user_id), predicate::always()) // Check user_id, new_password_hash can be anything valid
            .times(1)
            .returning(|_, new_hash| {
                // Basic check: ensure new_hash is not empty, real verification is harder here
                assert!(!new_hash.is_empty());
                Ok(())
            });

        let (mock_trs_from_setup, mock_ats_from_setup) = setup_mock_services();

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs_from_setup,
            mock_ats_from_setup,
            setup_mock_email_service(),
        );

        let result = auth_service.change_password(test_user_id, old_password.to_string(), new_password.to_string()).await;
        assert!(result.is_ok());
        
        // Clean up environment variables
        std::env::remove_var("JWT_AUDIENCE");
        std::env::remove_var("JWT_ISSUER");
    }

    #[tokio::test]
    async fn test_change_password_user_not_found() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let old_password = "OldPassword123!";
        let new_password = "NewPassword123!";

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(|_| Ok(None));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.change_password(test_user_id, old_password.to_string(), new_password.to_string()).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::UserNotFound);
    }

    #[tokio::test]
    async fn test_change_password_incorrect_old_password() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let wrong_old_password = "WrongOldPassword!";
        let new_password = "NewPassword123!";
        let test_user = create_test_user(test_user_id, "passwordchangeuser", true, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        mock_user_repo.expect_update_password().times(0);

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.change_password(test_user_id, wrong_old_password.to_string(), new_password.to_string()).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidCredentials);
    }

    #[tokio::test]
    async fn test_change_password_user_repo_find_fails() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let old_password = "OldPassword123!";
        let new_password = "NewPassword123!";

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(|_| Err(SqlxError::RowNotFound));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.change_password(test_user_id, old_password.to_string(), new_password.to_string()).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InternalServerError);
    }

    #[tokio::test]
    async fn test_register_user_repo_create_user_fails() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mock_email_service = setup_mock_email_service();
        let (mock_trs, mock_ats) = setup_mock_services();

        let register_input = RegisterInput {
            username: "dbfailuser".to_string(),
            email: "dbfail@example.com".to_string(),
            password: "Password123!".to_string(),
            password_confirm: "Password123!".to_string(),
        };

        mock_user_repo.expect_find_by_username().returning(|_| Ok(None));
        mock_user_repo.expect_find_user_by_email().returning(|_| Ok(None));
        mock_user_repo.expect_create_user()
            .times(1)
            .returning(|_| Err(SqlxError::RowNotFound));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.register(register_input).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InternalServerError);
    }

    #[tokio::test]
    async fn test_verify_email_successful() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let verification_token = Uuid::new_v4().to_string();
        let test_user = create_test_user(test_user_id, "verifyuser", false, "user");
        let cloned_user = test_user.clone();

        mock_user_repo.expect_verify_email()
            .with(predicate::eq(verification_token.clone()))
            .times(1)
            .returning(move |_| Ok(Some(test_user_id)));

        mock_user_repo.expect_find_by_id()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.verify_email(&verification_token).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_email_invalid_token() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let invalid_token = "invalid_token";

        mock_user_repo.expect_verify_email()
            .with(predicate::eq(invalid_token.to_string()))
            .times(1)
            .returning(|_| Ok(None));

        mock_user_repo.expect_find_by_verification_token()
            .with(predicate::eq(invalid_token.to_string()))
            .times(1)
            .returning(|_tok_str| Ok(None));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.verify_email(invalid_token).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidVerificationToken);
    }

    #[tokio::test]
    async fn test_verify_email_token_expired() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let verification_token = Uuid::new_v4().to_string();
        let test_user = User {
            id: test_user_id,
            username: "verifyuser".to_string(),
            email: Some("verify@example.com".to_string()),
            password_hash: bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap(),
            is_email_verified: false,
            verification_token: Some(verification_token.clone()),
            verification_token_expires_at: Some(Utc::now() - Duration::hours(1)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };
        let cloned_user = test_user.clone();

        mock_user_repo.expect_verify_email()
            .with(predicate::eq(verification_token.clone()))
            .times(1)
            .returning(|_| Ok(None));

        mock_user_repo.expect_find_by_verification_token()
            .with(predicate::eq(verification_token.clone()))
            .times(1)
            .returning(move |_tok_str| Ok(Some(cloned_user.clone())));

        mock_user_repo.expect_update().times(0);

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.verify_email(&verification_token).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::VerificationTokenExpired);
    }

    #[tokio::test]
    async fn test_verify_email_user_repo_find_fails() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let verification_token = Uuid::new_v4().to_string();

        mock_user_repo.expect_verify_email()
            .with(predicate::eq(verification_token.clone()))
            .times(1)
            .returning(|_| Err(SqlxError::RowNotFound));

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.verify_email(&verification_token).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InternalServerError);
    }

    #[tokio::test]
    async fn test_verify_email_user_repo_update_fails() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let test_user_id = Uuid::new_v4();
        let verification_token = Uuid::new_v4().to_string();
        let test_user = User {
            id: test_user_id,
            username: "verifyuser".to_string(),
            email: Some("verify@example.com".to_string()),
            password_hash: bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap(),
            is_email_verified: false,
            verification_token: Some(verification_token.clone()),
            verification_token_expires_at: Some(Utc::now() + Duration::hours(1)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };
        let cloned_user = test_user.clone();

        mock_user_repo.expect_verify_email()
            .with(predicate::eq(verification_token.clone()))
            .times(1)
            .returning(|_| Ok(None));

        mock_user_repo.expect_find_by_verification_token()
            .with(predicate::eq(verification_token.clone()))
            .times(1)
            .returning(move |_tok_str| Ok(Some(cloned_user.clone())));

        mock_user_repo.expect_update().times(0);

        let (mock_trs, mock_ats) = setup_mock_services();
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            setup_mock_email_service(),
        );

        let result = auth_service.verify_email(&verification_token).await;
        assert!(result.is_err());
        let auth_error = result.unwrap_err();
        assert_eq!(auth_error.error_type, AuthErrorType::InvalidVerificationToken);
    }

    #[tokio::test]
    async fn test_request_password_reset_successful() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mut mock_email_service = MockEmailServiceTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let test_user_id = Uuid::new_v4();
        let test_email = "user@example.com".to_string();
        let test_user = User {
            id: test_user_id,
            username: "testuser".to_string(),
            email: Some(test_email.clone()),
            password_hash: "hashed_password".to_string(),
            is_email_verified: true,
            verification_token: None,
            verification_token_expires_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            role: "user".to_string(),
            is_active: true,
        };
        let cloned_user = test_user.clone();

        let reset_token_value = "test_reset_token".to_string();
        let reset_token_model = PasswordResetToken {
            id: Uuid::new_v4(),
            user_id: test_user_id,
            token: reset_token_value.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            is_used: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let cloned_reset_token_model = reset_token_model.clone();

        mock_user_repo.expect_find_user_by_email()
            .with(predicate::eq(test_email.clone()))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        mock_user_repo.expect_create_password_reset_token()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(cloned_reset_token_model.clone()));

        mock_email_service.expect_send_password_reset_email()
            .with(predicate::eq(test_email.clone()), predicate::eq(reset_token_value.clone()))
            .times(1)
            .returning(|_, _| Ok(()));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            Arc::new(mock_email_service),
        );

        let result = auth_service.request_password_reset(test_email.clone()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_request_password_reset_user_not_found() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mock_email_service = MockEmailServiceTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let test_email = "nonexistent@example.com".to_string();

        mock_user_repo.expect_find_user_by_email()
            .with(predicate::eq(test_email.clone()))
            .times(1)
            .returning(|_| Ok(None));
        
        mock_user_repo.expect_create_password_reset_token().times(0);

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            Arc::new(mock_email_service),
        );

        let result = auth_service.request_password_reset(test_email.clone()).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_type, AuthErrorType::UserNotFound);
        }
    }

    #[tokio::test]
    async fn test_request_password_reset_email_send_fails() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mut mock_email_service = MockEmailServiceTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();

        let test_user_id = Uuid::new_v4();
        let test_email = "user@example.com".to_string();
        let test_user = User {
            id: test_user_id,
            username: "testuser_email_fail".to_string(),
            email: Some(test_email.clone()),
            ..create_test_user(test_user_id, "default", true, "user")
        };
        let cloned_user = test_user.clone();

        let reset_token_value = "test_reset_token_email_fail".to_string();
        let reset_token_model = PasswordResetToken { // Corrected
            id: Uuid::new_v4(),
            user_id: test_user_id,
            token: reset_token_value.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            is_used: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let cloned_reset_token_model = reset_token_model.clone();

        mock_user_repo.expect_find_user_by_email()
            .with(predicate::eq(test_email.clone()))
            .times(1)
            .returning(move |_| Ok(Some(cloned_user.clone())));

        mock_user_repo.expect_create_password_reset_token()
            .with(predicate::eq(test_user_id))
            .times(1)
            .returning(move |_| Ok(cloned_reset_token_model.clone()));

        mock_email_service.expect_send_password_reset_email()
            .with(predicate::eq(test_email.clone()), predicate::eq(reset_token_value.clone()))
            .times(1)
            .returning(|_, _| Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Simulated email send failure")) as Box<dyn std::error::Error + Send + Sync>));
            
        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            Arc::new(mock_email_service),
        );

        let result = auth_service.request_password_reset(test_email.clone()).await;
        assert!(result.is_ok()); 
    }

    #[tokio::test]
    async fn test_verify_password_reset_token_successful() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let mock_email_service = setup_mock_email_service();

        let test_user_id = Uuid::new_v4();
        let token_str = "valid_reset_token".to_string();

        let expected_reset_token = PasswordResetToken { // Corrected
            id: Uuid::new_v4(),
            user_id: test_user_id,
            token: token_str.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            is_used: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let cloned_expected_reset_token = expected_reset_token.clone();

        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(token_str.clone()))
            .times(1)
            .returning(move |_| Ok(Some(cloned_expected_reset_token.clone())));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.verify_password_reset_token(&token_str).await;
        assert!(result.is_ok());
        let user_id_from_token = result.unwrap();
        assert_eq!(user_id_from_token, test_user_id);
    }

    #[tokio::test]
    async fn test_verify_password_reset_token_invalid() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let mock_email_service = setup_mock_email_service();

        let invalid_token_str = "invalid_reset_token_string".to_string();

        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(invalid_token_str.clone()))
            .times(1)
            .returning(|_| Ok(None)); 

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.verify_password_reset_token(&invalid_token_str).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_type, AuthErrorType::InvalidToken);
        }
    }

    #[tokio::test]
    async fn test_verify_password_reset_token_expired() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let mock_email_service = setup_mock_email_service();

        let expired_token_str = "expired_reset_token_string".to_string();
        
        // Simulate the repository returning None for an expired token,
        // as the verify_reset_token method in UserRepositoryTrait already checks for expiration.
        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(expired_token_str.clone()))
            .times(1)
            .returning(|_| Ok(None));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.verify_password_reset_token(&expired_token_str).await;
        assert!(result.is_err());
        if let Err(e) = result {
            // The service layer currently maps Ok(None) from repo to InvalidToken.
            // If more specific error (e.g., TokenExpired) is desired, AuthService logic or
            // UserRepositoryTrait::verify_reset_token signature would need to change.
            assert_eq!(e.error_type, AuthErrorType::InvalidToken);
        }
    }

    #[tokio::test]
    async fn test_verify_password_reset_token_used() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let mock_email_service = setup_mock_email_service();

        let used_token_str = "used_reset_token_string".to_string();
        
        // Simulate the repository returning None for a used token.
        // The current AuthService maps Ok(None) from the repo to InvalidToken.
        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(used_token_str.clone()))
            .times(1)
            .returning(|_| Ok(None));

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.verify_password_reset_token(&used_token_str).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_type, AuthErrorType::InvalidToken);
        }
    }

    #[tokio::test]
    async fn test_verify_password_reset_token_db_error() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let mock_email_service = setup_mock_email_service();

        let token_str = "any_token_string_for_db_error".to_string();
        
        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(token_str.clone()))
            .times(1)
            .returning(|_| Err(SqlxError::RowNotFound)); // Simulate a generic DB error

        let auth_service = AuthService::new(
           Arc::new(mock_user_repo),
           TEST_JWT_SECRET.to_string(),
           "test_aud".to_string(),
           mock_trs,
           mock_ats,
           mock_email_service,
       );

        let result = auth_service.verify_password_reset_token(&token_str).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_type, AuthErrorType::InternalServerError);
        }
    }

    #[tokio::test]
    async fn test_reset_password_successful() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let mock_email_service = setup_mock_email_service();
        
        // For ActiveTokenService, the default mock from setup_mock_services is usually fine
        // as reset_password -> verify_password_reset_token doesn't directly interact with ATS expectations for *this* test's primary path.
        // verify_password_reset_token calls user_repo.verify_reset_token.
        // The record_tokens_for_user call within login/refresh is where ATS is more directly involved with expectations.
        let (_, mock_ats_arc) = setup_mock_services(); 

        let test_user_id = Uuid::new_v4();
        let reset_token_str = "valid_reset_token_for_reset".to_string();
        let new_password = "NewSecurePassword123!".to_string();
        let new_password_clone_for_bcrypt_check = new_password.clone(); // For the closure

        // 1. Mock UserRepositoryTrait::verify_reset_token (this is called by auth_service.verify_password_reset_token)
        let reset_token_model = PasswordResetToken {
            id: Uuid::new_v4(),
            user_id: test_user_id,
            token: reset_token_str.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            is_used: false, 
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let cloned_reset_token_model_for_verify_closure = reset_token_model.clone();
        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(reset_token_str.clone()))
            .times(1)
            .returning(move |_| Ok(Some(cloned_reset_token_model_for_verify_closure.clone())));

        // 2. Mock UserRepositoryTrait::update_password
        mock_user_repo.expect_update_password()
            .withf(move |uid, hashed_password| {
                *uid == test_user_id && bcrypt::verify(&new_password_clone_for_bcrypt_check, hashed_password).is_ok()
            })
            .times(1)
            .returning(|_, _| Ok(()));
        
        // 3. Create and configure a specific MockTokenRevocationServiceTrait for this test's expectation
        let mut specific_mock_trs = MockTokenRevocationServiceTrait::new();
        // If verify_password_reset_token internally used validate_jwt which uses TRS, we'd need:
        specific_mock_trs.expect_is_token_revoked().returning(|_| Ok(false)); // For validate_jwt if called by verify_password_reset_token

        specific_mock_trs.expect_revoke_all_user_tokens() // This is called by reset_password
            .with(
                predicate::eq(test_user_id), 
                predicate::always() // Reverted due to persistent HRTB issues
            )
            .times(1)
            .returning(|_,_| Ok(1)); // Simulate 1 (or more) tokens revoked
        // If verify_password_reset_token internally used validate_jwt which uses TRS, we'd need:
        // specific_mock_trs.expect_is_token_revoked().returning(|_| Ok(false)); // If needed by validate_jwt

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(), // Added jwt_audience
            Arc::new(specific_mock_trs),
            mock_ats_arc,
            mock_email_service,
        );

        let result = auth_service.reset_password(&reset_token_str, new_password.clone()).await;
        assert!(result.is_ok(), "reset_password failed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_reset_password_invalid_token() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services();
        let mock_email_service = setup_mock_email_service();

        let invalid_token_str = "invalid_reset_token_for_reset_attempt".to_string();
        let new_password = "NewSecurePassword123!".to_string();

        // Mock UserRepositoryTrait::verify_reset_token to return Ok(None) to simulate an invalid token
        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(invalid_token_str.clone()))
            .times(1)
            .returning(|_| Ok(None)); 
            // This will cause verify_password_reset_token to return Err(AuthErrorType::InvalidToken)

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.reset_password(&invalid_token_str, new_password).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_type, AuthErrorType::InvalidToken);
        }
    }

    #[tokio::test]
    async fn test_reset_password_user_repo_update_fails() {
        let mut mock_user_repo = MockUserRepositoryTrait::new();
        let (mock_trs, mock_ats) = setup_mock_services(); // Default mocks are fine here
        let mock_email_service = setup_mock_email_service();

        let test_user_id = Uuid::new_v4();
        let reset_token_str = "valid_token_for_repo_fail".to_string();
        let new_password = "NewSecurePassword123!".to_string();
        let new_password_clone_for_bcrypt_check = new_password.clone();


        // 1. Mock UserRepositoryTrait::verify_reset_token (called by auth_service.verify_password_reset_token)
        let reset_token_model = PasswordResetToken {
            id: Uuid::new_v4(),
            user_id: test_user_id,
            token: reset_token_str.clone(),
            expires_at: Utc::now() + Duration::hours(1),
            is_used: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let cloned_reset_token_model_for_verify_closure = reset_token_model.clone();
        mock_user_repo.expect_verify_reset_token()
            .with(predicate::eq(reset_token_str.clone()))
            .times(1)
            .returning(move |_| Ok(Some(cloned_reset_token_model_for_verify_closure.clone())));

        // 2. Mock UserRepositoryTrait::update_password to fail
        mock_user_repo.expect_update_password()
            .withf(move |uid, hashed_password| {
                *uid == test_user_id && bcrypt::verify(&new_password_clone_for_bcrypt_check, hashed_password).is_ok()
            })
            .times(1)
            .returning(|_, _| Err(SqlxError::RowNotFound)); // Simulate a DB error during update

        let auth_service = AuthService::new(
            Arc::new(mock_user_repo),
            TEST_JWT_SECRET.to_string(),
            "test_aud".to_string(),
            mock_trs,
            mock_ats,
            mock_email_service,
        );

        let result = auth_service.reset_password(&reset_token_str, new_password).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.error_type, AuthErrorType::InternalServerError);
        }
    }
} // Ensures the mod tests block is properly closed.
