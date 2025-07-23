//! Comprehensive API handler integration tests
//! Tests all HTTP endpoints with various scenarios and edge cases

use actix_web::{test, web, App, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use oxidizedoasis_websands::api::handlers::user_handler::{
    create_handler, create_user_handler, login_user_handler, verify_email_handler,
    get_current_user_handler, update_user_handler, delete_user_handler,
    request_password_reset_handler, reset_password_handler, refresh_token_handler,
    logout_user_handler
};
use oxidizedoasis_websands::core::user::{UserRepository, UserService};
use oxidizedoasis_websands::core::auth::{AuthService};
use oxidizedoasis_websands::core::auth::active_token::ActiveTokenService;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService;
use oxidizedoasis_websands::core::email::service::EmailService;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::common::validation::{UserInput, LoginInput};

mod common;
use common::{
    create_test_app_config, create_test_user, generate_test_token,
    test_data::*, http::*, env::with_env_vars, mocks::*
};

/// Test structure for user registration
#[derive(serde::Serialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
}

/// Test structure for login request
#[derive(serde::Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

/// Test structure for password reset request
#[derive(serde::Serialize)]
struct PasswordResetRequest {
    email: String,
}

/// Test fixture for API handler tests
struct ApiTestFixture<S>
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    app: S,
    test_user_id: Uuid,
    test_admin_id: Uuid,
    test_user_token: String,
    test_admin_token: String,
}

impl<S> ApiTestFixture<S>
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >,
{
    async fn new() -> ApiTestFixture<impl actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    >> {
        let config = create_test_app_config();
        let test_user_id = Uuid::new_v4();
        let test_admin_id = Uuid::new_v4();
        
        let test_user_token = generate_test_token(test_user_id, "user", 3600)
            .expect("Failed to generate user token");
        let test_admin_token = generate_test_token(test_admin_id, "admin", 3600)
            .expect("Failed to generate admin token");

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations for test users
        let test_user = create_test_user(test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let auth_service = Arc::new(AuthService::new(
            Arc::new(user_repo),
            common::TEST_JWT_SECRET.to_string(),
            common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_handler = create_handler(
            sqlx::PgPool::connect("postgresql://test:test@localhost/test").await
                .unwrap_or_else(|_| panic!("Could not connect to test database")),
            email_service,
            auth_service,
            token_revocation_service,
            active_token_service,
        );

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(config))
                .service(
                    web::scope("/users")
                        .route("/register", web::post().to(create_user_handler))
                        .route("/login", web::post().to(login_user_handler))
                        .route("/verify", web::get().to(verify_email_handler))
                        .route("/refresh", web::post().to(refresh_token_handler))
                        .route("/logout", web::post().to(logout_user_handler))
                        .service(
                            web::scope("/password-reset")
                                .route("/request", web::post().to(request_password_reset_handler))
                                .route("/reset", web::post().to(reset_password_handler))
                        )
                )
                .service(
                    web::scope("/api/users")
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;

        Self {
            app,
            test_user_id,
            test_admin_id,
            test_user_token,
            test_admin_token,
        }
    }
}

#[cfg(test)]
mod user_registration_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_user_registration_success() {
        let fixture = ApiTestFixture::new().await;
        
        let register_data = RegisterRequest {
            username: "newuser".to_string(),
            email: "newuser@example.com".to_string(), 
            password: "SecurePassword123!".to_string(),
            password_confirm: "SecurePassword123!".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(&register_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["user"].is_object());
        assert_eq!(body["data"]["user"]["username"], "newuser");
        assert_eq!(body["data"]["user"]["email"], "newuser@example.com");
        assert_eq!(body["data"]["user"]["is_email_verified"], false);
    }

    #[actix_rt::test]
    async fn test_user_registration_invalid_email() {
        let fixture = ApiTestFixture::new().await;
        
        let register_data = RegisterRequest {
            username: "testuser".to_string(),
            email: INVALID_EMAIL.to_string(),
            password: "SecurePassword123!".to_string(),
            password_confirm: "SecurePassword123!".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(&register_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("email"));
    }

    #[actix_rt::test]
    async fn test_user_registration_weak_password() {
        let fixture = ApiTestFixture::new().await;
        
        let register_data = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: WEAK_PASSWORD.to_string(),
            password_confirm: WEAK_PASSWORD.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(&register_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("password"));
    }

    #[actix_rt::test]
    async fn test_user_registration_password_mismatch() {
        let fixture = ApiTestFixture::new().await;
        
        let register_data = RegisterRequest {
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password: "SecurePassword123!".to_string(),
            password_confirm: "DifferentPassword123!".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(&register_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("match"));
    }
}

#[cfg(test)]
mod user_authentication_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_user_login_success() {
        let fixture = ApiTestFixture::new().await;
        
        let login_data = LoginRequest {
            username: TEST_USER_USERNAME.to_string(),
            password: TEST_USER_PASSWORD.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/login")
            .set_json(&login_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["access_token"].is_string());
        assert!(body["data"]["refresh_token"].is_string());
        assert!(body["data"]["user"].is_object());
        assert_eq!(body["data"]["user"]["username"], TEST_USER_USERNAME);
    }

    #[actix_rt::test]
    async fn test_user_login_invalid_credentials() {
        let fixture = ApiTestFixture::new().await;
        
        let login_data = LoginRequest {
            username: TEST_USER_USERNAME.to_string(),
            password: "WrongPassword".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/login")
            .set_json(&login_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("Invalid"));
    }

    #[actix_rt::test]
    async fn test_user_login_nonexistent_user() {
        let fixture = ApiTestFixture::new().await;
        
        let login_data = LoginRequest {
            username: "nonexistentuser".to_string(),
            password: TEST_USER_PASSWORD.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/login")
            .set_json(&login_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }

    #[actix_rt::test]
    async fn test_get_current_user_success() {
        let fixture = ApiTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/users/me", &fixture.test_user_token)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["user"].is_object());
        assert_eq!(body["data"]["user"]["id"], fixture.test_user_id.to_string());
    }

    #[actix_rt::test]
    async fn test_get_current_user_invalid_token() {
        let fixture = ApiTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/users/me", "invalid.jwt.token")
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }

    #[actix_rt::test]
    async fn test_get_current_user_missing_token() {
        let fixture = ApiTestFixture::new().await;
        
        let req = test::TestRequest::get()
            .uri("/api/users/me")
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

#[cfg(test)]
mod password_reset_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_request_password_reset_success() {
        let fixture = ApiTestFixture::new().await;
        
        let reset_data = PasswordResetRequest {
            email: TEST_USER_EMAIL.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/password-reset/request")
            .set_json(&reset_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("instructions"));
    }

    #[actix_rt::test]
    async fn test_request_password_reset_nonexistent_email() {
        let fixture = ApiTestFixture::new().await;
        
        let reset_data = PasswordResetRequest {
            email: "nonexistent@example.com".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/password-reset/request")
            .set_json(&reset_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        // Should return success to prevent email enumeration
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
    }

    #[actix_rt::test]
    async fn test_reset_password_success() {
        let fixture = ApiTestFixture::new().await;
        
        let reset_data = json!({
            "token": "valid_reset_token",
            "new_password": "NewSecurePassword123!",
            "confirm_password": "NewSecurePassword123!"
        });

        let req = test::TestRequest::post()
            .uri("/users/password-reset/reset")
            .set_json(&reset_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("reset successfully"));
    }

    #[actix_rt::test]
    async fn test_reset_password_password_mismatch() {
        let fixture = ApiTestFixture::new().await;
        
        let reset_data = json!({
            "token": "valid_reset_token",
            "new_password": "NewSecurePassword123!",
            "confirm_password": "DifferentPassword123!"
        });

        let req = test::TestRequest::post()
            .uri("/users/password-reset/reset")
            .set_json(&reset_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("match"));
    }
}

#[cfg(test)]
mod user_management_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_user_success() {
        let fixture = ApiTestFixture::new().await;
        
        let update_data = json!({
            "username": "updateduser",
            "email": "updated@example.com"
        });

        let req = create_auth_request("PUT", &format!("/api/users/{}", fixture.test_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["user"]["username"], "updateduser");
    }

    #[actix_rt::test]
    async fn test_update_user_unauthorized_other_user() {
        let fixture = ApiTestFixture::new().await;
        let other_user_id = Uuid::new_v4();
        
        let update_data = json!({
            "username": "hacker"
        });

        let req = create_auth_request("PUT", &format!("/api/users/{}", other_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("denied"));
    }

    #[actix_rt::test]
    async fn test_delete_user_success() {
        let fixture = ApiTestFixture::new().await;
        
        let req = create_auth_request("DELETE", &format!("/api/users/{}", fixture.test_user_id), &fixture.test_user_token)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("deleted"));
    }

    #[actix_rt::test]
    async fn test_delete_user_unauthorized_other_user() {
        let fixture = ApiTestFixture::new().await;
        let other_user_id = Uuid::new_v4();
        
        let req = create_auth_request("DELETE", &format!("/api/users/{}", other_user_id), &fixture.test_user_token)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }
}

#[cfg(test)]
mod token_management_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_refresh_token_success() {
        let fixture = ApiTestFixture::new().await;
        
        let refresh_data = json!({
            "token": "valid_refresh_token"
        });

        let req = test::TestRequest::post()
            .uri("/users/refresh")
            .set_json(&refresh_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["access_token"].is_string());
        assert!(body["data"]["refresh_token"].is_string());
    }

    #[actix_rt::test]
    async fn test_refresh_token_invalid() {
        let fixture = ApiTestFixture::new().await;
        
        let refresh_data = json!({
            "token": "invalid_refresh_token"
        });

        let req = test::TestRequest::post()
            .uri("/users/refresh")
            .set_json(&refresh_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }

    #[actix_rt::test]
    async fn test_logout_success() {
        let fixture = ApiTestFixture::new().await;
        
        let logout_data = json!({
            "token": "valid_refresh_token"
        });

        let req = create_auth_request("POST", "/users/logout", &fixture.test_user_token)
            .set_json(&logout_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("Logged out"));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_malformed_json_request() {
        let fixture = ApiTestFixture::new().await;
        
        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_payload("{invalid json")
            .insert_header(("content-type", "application/json"))
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn test_missing_required_fields() {
        let fixture = ApiTestFixture::new().await;
        
        let incomplete_data = json!({
            "username": "testuser"
            // Missing email, password, etc.
        });

        let req = test::TestRequest::post()
            .uri("/users/register")
            .set_json(&incomplete_data)
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn test_route_not_found() {
        let fixture = ApiTestFixture::new().await;
        
        let req = test::TestRequest::get()
            .uri("/nonexistent/route")
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_method_not_allowed() {
        let fixture = ApiTestFixture::new().await;
        
        let req = test::TestRequest::patch()
            .uri("/users/register")
            .to_request();

        let resp = test::call_service(&fixture.app, req).await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}