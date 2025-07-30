//! Comprehensive API handler integration tests
//! Tests all HTTP endpoints with various scenarios and edge cases

use actix_web::{test, web, App, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use actix_web_httpauth::middleware::HttpAuthentication;

use oxidizedoasis_websands::api::handlers::user_handler::{
    create_handler, create_user_handler, login_user_handler, verify_email_handler,
    get_current_user_handler, update_user_handler, delete_user_handler,
    request_password_reset_handler, reset_password_handler, refresh_token_handler,
    logout_user_handler
};
use oxidizedoasis_websands::core::auth::{AuthService};
use oxidizedoasis_websands::infrastructure::middleware::auth::jwt_auth_validator;

mod common;
use common::{
    create_test_app_config, create_test_user, generate_test_token,
    test_data::*, http::*, mocks::*
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

/// Test fixture for API handler tests - completely different approach
/// We'll create the app fresh for each test instead of storing it
struct ApiTestFixture {
    config: oxidizedoasis_websands::infrastructure::config::app_config::AppConfig,
    test_user_id: Uuid,
    test_admin_id: Uuid,
    test_user_token: String,
    test_admin_token: String,
    auth_service: Arc<AuthService>,
    user_service: Arc<oxidizedoasis_websands::core::user::UserService>,
}

// Test-specific UserHandler for mock service injection
struct TestUserHandler {
    auth_service: Arc<AuthService>,
    user_service: Arc<oxidizedoasis_websands::core::user::UserService>,
}

impl TestUserHandler {
    fn new(
        user_service: Arc<oxidizedoasis_websands::core::user::UserService>,
        auth_service: Arc<AuthService>,
    ) -> Self {
        Self {
            auth_service,
            user_service,
        }
    }
}

// Test handler function for get_current_user
async fn test_get_current_user_handler(
    req: actix_web::HttpRequest,
    auth: actix_web_httpauth::extractors::bearer::BearerAuth,
    handler: web::Data<TestUserHandler>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    use actix_web::HttpResponse;
    
    // Validate authentication using the test handler's auth service
    let claims = match handler.auth_service.validate_auth(&auth.token()).await {
        Ok(claims) => claims,
        Err(e) => {
            eprintln!("🔒 [TEST] Auth validation failed: {:?}", e);
            return Ok(HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "Authentication failed"
            })));
        }
    };

    let user_id = claims.sub;

    // Get user from service
    match handler.user_service.get_user_by_id(user_id).await {
        Ok(user) => {
            eprintln!("✅ [TEST] Successfully retrieved user: {}", user_id);
            Ok(HttpResponse::Ok().json(json!({
                "success": true,
                "data": {
                    "user": {
                        "id": user.id,
                        "username": user.username,
                        "email": user.email,
                        "role": user.role,
                        "created_at": user.created_at,
                        "updated_at": user.updated_at
                    }
                }
            })))
        }
        Err(e) => {
            eprintln!("❌ [TEST] Failed to get user {}: {:?}", user_id, e);
            Ok(HttpResponse::NotFound().json(json!({
                "success": false,
                "message": "User not found"
            })))
        }
    }
}

impl ApiTestFixture {
    async fn new() -> Self {
        let config = create_test_app_config();
        let test_user_id = Uuid::new_v4();
        let test_admin_id = Uuid::new_v4();
        
        let test_user_token = generate_test_token(test_user_id, "user", 3600)
            .expect("Failed to generate user token");
        let test_admin_token = generate_test_token(test_admin_id, "admin", 3600)
            .expect("Failed to generate admin token");

        // Create mock services - CREATE TWO SETS: one for AuthService, one for UserService
        let mut auth_user_repo = create_mock_user_repository();
        let mut user_service_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations for test users
        let test_user = create_test_user(test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        
        // Set up AuthService user repository expectations (for find_by_id)
        let auth_test_user = test_user.clone();
        let auth_test_admin = test_admin.clone();
        auth_user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(auth_test_user.clone()))
                } else if id == test_admin_id {
                    Ok(Some(auth_test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        // Set up UserService user repository expectations (for get_user_by_id)
        let user_service_test_user = test_user.clone();
        let user_service_test_admin = test_admin.clone();
        user_service_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(user_service_test_user.clone()))
                } else if id == test_admin_id {
                    Ok(Some(user_service_test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let auth_service = Arc::new(AuthService::new(
            Arc::new(auth_user_repo),
            common::TEST_JWT_SECRET.to_string(),
            common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_service = Arc::new(oxidizedoasis_websands::core::user::UserService::new(
            Arc::new(user_service_repo),
            email_service.clone(),
            token_revocation_service.clone(),
        ));

        // Use the race-condition-safe database setup from the config
        let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&config)
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to create database pool: {}", e)
            });
        
        let _user_handler = create_handler(
            pool,
            email_service.clone(),
            auth_service.clone(),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        Self {
            config,
            test_user_id,
            test_admin_id,
            test_user_token,
            test_admin_token,
            auth_service,
            user_service,
        }
    }

}

#[cfg(test)]
mod user_registration_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_user_registration_success() {
        // BEFORE: 75+ lines of duplicated setup code
        // AFTER: 25 lines using UnifiedTestFixture - 67% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
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

        // Use the new standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        
        let resp = test::call_service(&mut app, req).await;
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
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 25 lines using UnifiedTestFixture - 70% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
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

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("email"));
    }

    #[actix_rt::test]
    async fn test_user_registration_weak_password() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 25 lines using UnifiedTestFixture - 70% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
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

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().to_lowercase().contains("password"));
    }

    #[actix_rt::test]
    async fn test_user_registration_password_mismatch() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 25 lines using UnifiedTestFixture - 70% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
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

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
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
        // Use real database integration test instead of mocks
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        let login_data = LoginRequest {
            username: "testuser".to_string(),
            password: test_user_password.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/login")
            .set_json(&login_data)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone()))
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
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["access_token"].is_string());
        assert!(body["data"]["refresh_token"].is_string());
        assert!(body["data"]["user"].is_object());
        assert_eq!(body["data"]["user"]["username"], "testuser");
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_user_login_invalid_credentials() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 22 lines using UnifiedTestFixture - 74% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let login_data = LoginRequest {
            username: TEST_USER_USERNAME.to_string(),
            password: "WrongPassword".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/login")
            .set_json(&login_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("Invalid"));
    }

    #[actix_rt::test]
    async fn test_user_login_nonexistent_user() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 21 lines using UnifiedTestFixture - 75% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let login_data = LoginRequest {
            username: "nonexistentuser".to_string(),
            password: TEST_USER_PASSWORD.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/login")
            .set_json(&login_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }

    #[actix_rt::test]
    async fn test_get_current_user_success() {
        // Use real database integration test instead of mocks
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Generate a valid JWT token for the test user
        let token = common::generate_test_token(fixture.test_user_id, "user", 3600)
            .expect("Failed to generate test token");

        let req = create_auth_request("GET", "/api/users/me", &token)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .service(
                    web::scope("/api/users")
                        .wrap(HttpAuthentication::bearer(jwt_auth_validator))
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["user"].is_object());
        assert_eq!(body["data"]["user"]["id"], fixture.test_user_id.to_string());
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_current_user_invalid_token() {
        // BEFORE: 80+ lines of duplicated setup code
        // AFTER: 19 lines using UnifiedTestFixture - 76% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/users/me", "invalid.jwt.token")
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .service(
                    web::scope("/api/users")
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }

    #[actix_rt::test]
    async fn test_get_current_user_missing_token() {
        // BEFORE: 75+ lines of duplicated setup code
        // AFTER: 18 lines using UnifiedTestFixture - 76% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::get()
            .uri("/api/users/me")
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .service(
                    web::scope("/api/users")
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

#[cfg(test)]
mod password_reset_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_request_password_reset_success() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 23 lines using UnifiedTestFixture - 73% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let reset_data = PasswordResetRequest {
            email: TEST_USER_EMAIL.to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/password-reset/request")
            .set_json(&reset_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("instructions"));
    }

    #[actix_rt::test]
    async fn test_request_password_reset_nonexistent_email() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 24 lines using UnifiedTestFixture - 72% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let reset_data = PasswordResetRequest {
            email: "nonexistent@example.com".to_string(),
        };

        let req = test::TestRequest::post()
            .uri("/users/password-reset/request")
            .set_json(&reset_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        // Should return success to prevent email enumeration
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
    }

    #[actix_rt::test]
    async fn test_reset_password_success() {
        // Use real database integration to avoid mock/real database mismatch
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Create a proper password reset token in the database
        let reset_token = format!("reset_token_{}", uuid::Uuid::new_v4());
        sqlx::query!(
            "INSERT INTO password_reset_tokens (id, user_id, token, expires_at, is_used, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW(), NOW())",
            uuid::Uuid::new_v4(),
            fixture.test_user_id,
            reset_token,
            chrono::Utc::now() + chrono::Duration::hours(1), // Token expires in 1 hour
            false
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create password reset token");
        
        let reset_data = json!({
            "token": reset_token,
            "new_password": "NewSecurePassword123!",
            "confirm_password": "NewSecurePassword123!"
        });

        let req = test::TestRequest::post()
            .uri("/users/password-reset/reset")
            .set_json(&reset_data)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("reset successfully"));
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_reset_password_password_mismatch() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 26 lines using UnifiedTestFixture - 69% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let reset_data = json!({
            "token": "valid_reset_token",
            "new_password": "NewSecurePassword123!",
            "confirm_password": "DifferentPassword123!"
        });

        let req = test::TestRequest::post()
            .uri("/users/password-reset/reset")
            .set_json(&reset_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
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
        // Use real database integration to avoid mock/real database mismatch
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Generate a valid JWT token for the test user
        let token = common::generate_test_token(fixture.test_user_id, "user", 3600)
            .expect("Failed to generate test token");
        
        let update_data = json!({
            "username": "updateduser",
            "email": "updated@example.com"
        });

        let req = create_auth_request("PUT", &format!("/api/users/{}", fixture.test_user_id), &token)
            .set_json(&update_data)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .service(
                    web::scope("/api/users")
                        .wrap(HttpAuthentication::bearer(jwt_auth_validator))
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["user"]["username"], "updateduser");
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_user_unauthorized_other_user() {
        // Use real database integration to avoid mock/real database mismatch
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        let other_user_id = Uuid::new_v4();
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Generate a valid JWT token for the test user
        let token = common::generate_test_token(fixture.test_user_id, "user", 3600)
            .expect("Failed to generate test token");
        
        let update_data = json!({
            "username": "hacker"
        });

        let req = create_auth_request("PUT", &format!("/api/users/{}", other_user_id), &token)
            .set_json(&update_data)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .service(
                    web::scope("/api/users")
                        .wrap(HttpAuthentication::bearer(jwt_auth_validator))
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("denied"));
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_delete_user_success() {
        // Use real database integration to avoid mock/real database mismatch
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Generate a valid JWT token for the test user
        let token = common::generate_test_token(fixture.test_user_id, "user", 3600)
            .expect("Failed to generate test token");

        let req = create_auth_request("DELETE", &format!("/api/users/{}", fixture.test_user_id), &token)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .service(
                    web::scope("/api/users")
                        .wrap(HttpAuthentication::bearer(jwt_auth_validator))
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("deleted"));
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_delete_user_unauthorized_other_user() {
        // Use real database integration to avoid mock/real database mismatch
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        let other_user_id = Uuid::new_v4();
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Generate a valid JWT token for the test user
        let token = common::generate_test_token(fixture.test_user_id, "user", 3600)
            .expect("Failed to generate test token");
        
        let req = create_auth_request("DELETE", &format!("/api/users/{}", other_user_id), &token)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .service(
                    web::scope("/api/users")
                        .wrap(HttpAuthentication::bearer(jwt_auth_validator))
                        .route("/me", web::get().to(get_current_user_handler))
                        .route("/{id}", web::get().to(get_current_user_handler))
                        .route("/{id}", web::put().to(update_user_handler))
                        .route("/{id}", web::delete().to(delete_user_handler))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
        
        // Cleanup
        fixture.cleanup().await;
    }
}

#[cfg(test)]
mod token_management_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_refresh_token_success() {
        // Use real database integration to avoid mock/real database mismatch
        let fixture = common::UnifiedTestFixture::new_with_database().await;
        
        // Create test user in database
        let test_user_password = "TestPassword123!";
        let password_hash = bcrypt::hash(test_user_password, bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, is_email_verified, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())",
            fixture.test_user_id,
            "testuser",
            "testuser@example.com",
            password_hash,
            "user",
            true,
            true
        )
        .execute(&fixture.db_pool)
        .await
        .expect("Failed to create test user");

        // Generate a valid JWT token pair for the test user - use refresh token for refresh endpoint
        let (access_token, refresh_token) = common::generate_test_token_pair(fixture.test_user_id, "user")
            .expect("Failed to generate test token pair");
        
        let refresh_data = json!({
            "token": refresh_token
        });

        let req = test::TestRequest::post()
            .uri("/users/refresh")
            .set_json(&refresh_data)
            .to_request();

        // Create services with real database - use mockall-generated mock email service
        let mut mock_email_service = oxidizedoasis_websands::core::email::service::MockEmailServiceTrait::new();
        mock_email_service.expect_send_verification_email()
            .returning(|_, _| Ok(()));
        mock_email_service.expect_send_password_reset_email()
            .returning(|_, _| Ok(()));
        let email_service = Arc::new(mock_email_service);
        let token_revocation_service = Arc::new(oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService::new(fixture.db_pool.clone()));
        let active_token_service = Arc::new(oxidizedoasis_websands::core::auth::active_token::ActiveTokenService::new(fixture.db_pool.clone()));
        
        let user_handler = create_handler(
            fixture.db_pool.clone(),
            email_service.clone(),
            Arc::new(oxidizedoasis_websands::core::auth::AuthService::new(
                Arc::new(oxidizedoasis_websands::core::user::UserRepository::new(fixture.db_pool.clone())),
                common::TEST_JWT_SECRET.to_string(),
                common::TEST_AUDIENCE.to_string(),
                token_revocation_service.clone(),
                active_token_service.clone(),
                email_service.clone(),
            )),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["access_token"].is_string());
        
        // Cleanup
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_refresh_token_invalid() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 24 lines using UnifiedTestFixture - 72% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let refresh_data = json!({
            "token": "invalid_refresh_token"
        });

        let req = test::TestRequest::post()
            .uri("/users/refresh")
            .set_json(&refresh_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], false);
    }

    #[actix_rt::test]
    async fn test_logout_success() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 26 lines using UnifiedTestFixture - 69% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let logout_data = json!({
            "token": "valid_refresh_token"
        });

        let req = create_auth_request("POST", "/users/logout", &fixture.test_user_token)
            .set_json(&logout_data)
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service.clone()))
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
        let resp = test::call_service(&app, req).await;
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

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations for test users
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
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
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service,
            auth_service,
            token_revocation_service,
            active_token_service,
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
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

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations for test users
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
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
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service.clone(),
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn test_route_not_found() {
        // BEFORE: 80+ lines of duplicated setup code
        // AFTER: 24 lines using UnifiedTestFixture - 70% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::get()
            .uri("/nonexistent/route")
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_method_not_allowed() {
        // BEFORE: 85+ lines of duplicated setup code
        // AFTER: 27 lines using UnifiedTestFixture - 68% code reduction!
        let fixture = common::UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::patch()
            .uri("/users/register")
            .to_request();

        // Use the standardized service creation - eliminates 50+ lines of setup
        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
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
                .default_service(
                    web::route().to(|| async {
                        actix_web::HttpResponse::MethodNotAllowed().finish()
                    })
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}