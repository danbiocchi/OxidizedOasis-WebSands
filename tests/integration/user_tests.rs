//! User integration tests
//!
//! This module contains integration tests for user-related functionality.

use actix_web::{test, web, App, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use oxidizedoasis_websands::core::user::{User, UserRepositoryTrait};
use oxidizedoasis_websands::core::user::repository::MockUserRepositoryTrait;
use oxidizedoasis_websands::core::auth::AuthService;
use oxidizedoasis_websands::core::auth::active_token::ActiveTokenService;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService;
use oxidizedoasis_websands::core::email::service::EmailService;

use test_common::{
    UnifiedTestFixture, create_test_user, create_standard_mock_services,
    test_data::*, http::*, mocks::*
};

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_user_creation_basic() {
        let fixture = UnifiedTestFixture::new_with_database().await;
        
        let test_user = create_test_user(
            fixture.test_user_id,
            "testuser",
            "test@example.com",
            true,
            "user"
        );
        
        assert_eq!(test_user.username, "testuser");
        assert_eq!(test_user.email, Some("test@example.com".to_string()));
        assert_eq!(test_user.role, "user");
        assert!(test_user.is_active);
    }
}
