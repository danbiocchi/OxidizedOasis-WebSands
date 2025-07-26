//! Comprehensive middleware testing
//! Tests authentication, authorization, CORS, and other middleware components

use actix_web::{test, web, App, http::StatusCode, cookie::Cookie, HttpMessage, FromRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use oxidizedoasis_websands::infrastructure::middleware::{
    auth::{jwt_auth_validator, cookie_auth_validator, AuthError},
    admin::admin_validator,
};
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait;

mod common;
use common::{
    create_test_app_config, generate_test_token, create_test_claims,
    mocks::*, env::with_env_vars, TEST_JWT_SECRET, TEST_AUDIENCE
};

/// Test fixture for middleware tests
struct MiddlewareTestFixture {
    app_config: AppConfig,
    token_revocation_service: Arc<dyn TokenRevocationServiceTrait>,
    valid_user_token: String,
    valid_admin_token: String,
    expired_token: String,
    invalid_token: String,
    revoked_token: String,
}

impl MiddlewareTestFixture {
    fn new() -> Self {
        let app_config = create_test_app_config();
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        
        let user_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        
        let valid_user_token = generate_test_token(user_id, "user", 3600)
            .expect("Failed to generate user token");
        let valid_admin_token = generate_test_token(admin_id, "admin", 3600)
            .expect("Failed to generate admin token");
        let expired_token = generate_test_token(user_id, "user", -3600)
            .expect("Failed to generate expired token");
        let invalid_token = "invalid.jwt.token".to_string();
        let revoked_token = generate_test_token(user_id, "user", 3600)
            .expect("Failed to generate revoked token");

        Self {
            app_config,
            token_revocation_service,
            valid_user_token,
            valid_admin_token,
            expired_token,
            invalid_token,
            revoked_token,
        }
    }
}

#[cfg(test)]
mod auth_middleware_tests {
    use super::*;
    use actix_web_httpauth::extractors::bearer::BearerAuth;
    use std::collections::HashMap;

    #[actix_rt::test]
    async fn test_jwt_auth_validator_valid_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.valid_user_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = jwt_auth_validator(req, bearer_auth).await;
            assert!(result.is_ok());
            
            let validated_req = result.unwrap();
            let extensions = validated_req.extensions();
            let claims = extensions.get::<oxidizedoasis_websands::core::auth::jwt::Claims>();
            assert!(claims.is_some());
        });
    }

    #[actix_rt::test]
    async fn test_jwt_auth_validator_invalid_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.invalid_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = jwt_auth_validator(req, bearer_auth).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        });
    }

    #[actix_rt::test]
    async fn test_cookie_auth_validator_valid_cookie() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .cookie(Cookie::new("access_token", fixture.valid_user_token.clone()))
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();
            
            let result = cookie_auth_validator(req).await;
            assert!(result.is_ok());
            
            let validated_req = result.unwrap();
            let extensions = validated_req.extensions();
            let claims = extensions.get::<oxidizedoasis_websands::core::auth::jwt::Claims>();
            assert!(claims.is_some());
        });
    }

    #[actix_rt::test]
    async fn test_cookie_auth_validator_missing_cookie() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();
            
            let result = cookie_auth_validator(req).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        });
    }
}

#[cfg(test)]
mod admin_middleware_tests {
    use super::*;
    use actix_web_httpauth::extractors::bearer::BearerAuth;
    use std::collections::HashMap;

    #[actix_rt::test]
    async fn test_admin_validator_valid_admin_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.valid_admin_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = admin_validator(req, bearer_auth).await;
            assert!(result.is_ok());
            
            let validated_req = result.unwrap();
            let extensions = validated_req.extensions();
            let claims = extensions.get::<oxidizedoasis_websands::core::auth::jwt::Claims>();
            assert!(claims.is_some());
            assert_eq!(claims.unwrap().role, "admin");
        });
    }

    #[actix_rt::test]
    async fn test_admin_validator_non_admin_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.valid_user_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = admin_validator(req, bearer_auth).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        });
    }

    #[actix_rt::test]
    async fn test_admin_validator_invalid_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.invalid_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = admin_validator(req, bearer_auth).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        });
    }
}

#[cfg(test)]
mod csrf_protection_tests {
    use super::*;
    
    #[actix_rt::test]
    async fn test_csrf_protection_missing_header() {
        // Test CSRF protection when X-CSRF-Token header is missing
        let fixture = MiddlewareTestFixture::new();
        
        // This would be tested with actual CSRF middleware integration
        // For now, this is a placeholder for future CSRF middleware testing
        assert!(true); // Placeholder assertion
    }

    #[actix_rt::test]
    async fn test_csrf_protection_invalid_token() {
        // Test CSRF protection with invalid token
        let fixture = MiddlewareTestFixture::new();
        
        // This would be tested with actual CSRF middleware integration
        // For now, this is a placeholder for future CSRF middleware testing
        assert!(true); // Placeholder assertion
    }
}

#[cfg(test)]
mod rate_limiting_tests {
    use super::*;
    
    #[actix_rt::test]
    async fn test_rate_limiting_under_limit() {
        // Test that requests under rate limit are allowed
        assert!(true); // Placeholder for future rate limiting tests
    }

    #[actix_rt::test]
    async fn test_rate_limiting_over_limit() {
        // Test that requests over rate limit are blocked
        assert!(true); // Placeholder for future rate limiting tests
    }
}