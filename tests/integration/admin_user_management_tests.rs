//! Comprehensive admin user management integration tests
//! Tests all admin user management endpoints with various scenarios and edge cases

use actix_web::{test, web, App, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use oxidizedoasis_websands::api::routes::admin::user_management::{
    list_users, get_user, update_user_role, update_user_username,
    update_user_status, delete_user
};
use oxidizedoasis_websands::core::auth::{AuthService};
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::infrastructure::middleware::admin_validator;
use actix_web_httpauth::middleware::HttpAuthentication;


use test_common::{
    test_data::*, http::*, mocks::*, create_test_user,
    EnhancedTestConfig, assert_user_response, assert_error_response, assert_user_in_database,
    seed_admin_test_data, UserManagementScenario, generate_test_token
};

/// Test structure for updating user role
#[derive(serde::Serialize)]
struct UpdateRoleRequest {
    role: String,
}

/// Test structure for updating username
#[derive(serde::Serialize)]
struct UpdateUsernameRequest {
    username: String,
}

/// Test structure for updating user status
#[derive(serde::Serialize)]
struct UpdateStatusRequest {
    is_active: bool,
}

#[cfg(test)]
mod admin_user_list_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_list_users_success_as_admin() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("GET", "/api/admin/users", &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(auth_service.clone()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["data"]["users"].is_array());
        let users = body["data"]["users"].as_array().unwrap();
        assert_eq!(users.len(), 3); // test_user, test_admin, test_target_user
    }

    #[actix_rt::test]
    async fn test_list_users_forbidden_as_user() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("GET", "/api/admin/users", &fixture.test_user_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(auth_service.clone()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_list_users_unauthorized_without_token() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = test::TestRequest::get()
            .uri("/api/admin/users")
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(auth_service.clone()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn test_list_users_invalid_token() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("GET", "/api/admin/users", "invalid.jwt.token")
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(auth_service.clone()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

#[cfg(test)]
mod admin_user_detail_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_get_user_success() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        // Create database helper and insert real users instead of using mocks
        let db_helper = test_common::database::DatabaseTestHelper::from_config(&fixture.config).await
            .expect("Failed to create database helper");
        
        // Clean up any existing test data
        db_helper.cleanup_all().await.expect("Failed to cleanup database");
        
        // Insert real users into the database using realistic password hashes
        let password_hash = bcrypt::hash("test_password", bcrypt::DEFAULT_COST).unwrap();
        
        // Insert test admin user (for authentication)
        db_helper.insert_test_user(
            fixture.test_admin_id,
            TEST_ADMIN_USERNAME,
            TEST_ADMIN_EMAIL,
            &password_hash,
            "admin",
            true,
            true,
        ).await.expect("Failed to insert test admin user");
        
        // Insert target user (the one we're trying to get)
        db_helper.insert_test_user(
            fixture.test_target_user_id,
            "targetuser",
            "target@example.com",
            &password_hash,
            "user",
            true,
            true,
        ).await.expect("Failed to insert test target user");

        let req = create_auth_request("GET", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_admin_token)
            .to_request();

        // Create real database pool and services
        let pool = db_helper.pool();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Create real user repository using the database pool
        let user_repo = Arc::new(oxidizedoasis_websands::core::user::repository::UserRepository::new((*pool).clone()));

        let auth_service = Arc::new(AuthService::new(
            user_repo.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            pool.as_ref().clone(),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        // Use the same real repository for admin routes
        let admin_user_repo: Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait> = user_repo.clone();

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["id"], fixture.test_target_user_id.to_string());
        assert_eq!(body["data"]["username"], "targetuser");
        assert_eq!(body["data"]["email"], "target@example.com");
        assert_eq!(body["data"]["role"], "user");
        
        // Cleanup after test
        db_helper.cleanup_all().await.expect("Failed to cleanup database");
    }

    #[actix_rt::test]
    async fn test_get_user_not_found() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        let non_existent_id = Uuid::new_v4();
        
        println!("[DEBUG] test_get_user_not_found: Starting test");
        println!("[DEBUG] test_get_user_not_found: Non-existent ID: {}", non_existent_id);
        let req = create_auth_request("GET", &format!("/api/admin/users/{}", non_existent_id), &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        println!("[DEBUG] test_get_user_not_found: Response status: {:?}", status);
        
        // Try to read response body to see what we actually get
        let body_bytes = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        println!("[DEBUG] test_get_user_not_found: Response body: {}", body_str);
        
        // Parse as JSON if possible
        if let Ok(body_json) = serde_json::from_str::<Value>(&body_str) {
            println!("[DEBUG] test_get_user_not_found: Parsed JSON: {}", body_json);
        }
        
        assert_eq!(status, StatusCode::NOT_FOUND);

        // For actual test, need to make another request since we consumed the body
        let req2 = create_auth_request("GET", &format!("/api/admin/users/{}", non_existent_id), &fixture.test_admin_token)
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        let body: Value = test::read_body_json(resp2).await;
        // ApiError responses don't have "success" field, they have "message" and "error_type"
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_get_user_forbidden_as_user() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("GET", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_user_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_get_user_invalid_uuid() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        println!("[DEBUG] test_get_user_invalid_uuid: Starting test");
        let req = create_auth_request("GET", "/api/admin/users/invalid-uuid", &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        println!("[DEBUG] test_get_user_invalid_uuid: Response status: {:?}", status);
        
        // Try to read response body to see what we actually get
        let body_bytes = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        println!("[DEBUG] test_get_user_invalid_uuid: Response body: {}", body_str);
        
        // Note: Actix-web returns 404 for invalid UUID in path parameter, not 400
        // This is expected framework behavior
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

#[cfg(test)]
mod admin_user_role_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_user_role_success() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["role"], "admin");
        assert_eq!(body["data"]["id"], fixture.test_target_user_id.to_string());
    }

    #[actix_rt::test]
    async fn test_update_user_role_invalid_role() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateRoleRequest {
            role: "invalid_role".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("Invalid role"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_self_edit_forbidden() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateRoleRequest {
            role: "user".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_not_found() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        let non_existent_id = Uuid::new_v4();
        
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", non_existent_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_forbidden_as_user() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_update_user_role_valid_roles() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        // Test both valid roles
        for role in &["user", "admin"] {
            let update_data = UpdateRoleRequest {
                role: role.to_string(),
            };

            let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
                .set_json(&update_data)
                .to_request();

            // Create standard mock services
            let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

            let admin_auth = HttpAuthentication::bearer(admin_validator);

            let mut app = test::init_service(
                App::new()
                    .app_data(web::Data::new(user_handler))
                    .app_data(web::Data::new(fixture.config.clone()))
                    .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                    .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                    .app_data(web::Data::new(auth_service.clone()))
                    .app_data(web::Data::new(admin_user_repo))
                    .service(
                        web::scope("/api/admin/users")
                            .wrap(admin_auth)
                            .route("", web::get().to(list_users))
                            .route("/{id}", web::get().to(get_user))
                            .route("/{id}/role", web::put().to(update_user_role))
                            .route("/{id}/username", web::put().to(update_user_username))
                            .route("/{id}/status", web::put().to(update_user_status))
                            .route("/{id}", web::delete().to(delete_user))
                    )
            ).await;
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["success"], true);
            assert_eq!(body["data"]["role"], *role);
        }
    }
}

#[cfg(test)]
mod admin_user_username_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_user_username_success() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        // Create database helper and insert real users instead of using mocks
        let db_helper = test_common::database::DatabaseTestHelper::from_config(&fixture.config).await
            .expect("Failed to create database helper");
        
        // Clean up any existing test data
        db_helper.cleanup_all().await.expect("Failed to cleanup database");
        
        // Insert real users into the database using realistic password hashes
        let password_hash = bcrypt::hash("test_password", bcrypt::DEFAULT_COST).unwrap();
        
        // Insert test admin user (for authentication)
        db_helper.insert_test_user(
            fixture.test_admin_id,
            TEST_ADMIN_USERNAME,
            TEST_ADMIN_EMAIL,
            &password_hash,
            "admin",
            true,
            true,
        ).await.expect("Failed to insert test admin user");
        
        // Insert target user (the one we're trying to update)
        db_helper.insert_test_user(
            fixture.test_target_user_id,
            "targetuser",
            "target@example.com",
            &password_hash,
            "user",
            true,
            true,
        ).await.expect("Failed to insert test target user");
        
        let update_data = UpdateUsernameRequest {
            username: "newusername".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create real database pool and services
        let pool = db_helper.pool();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Create real user repository using the database pool
        let user_repo = Arc::new(oxidizedoasis_websands::core::user::repository::UserRepository::new((*pool).clone()));

        let auth_service = Arc::new(AuthService::new(
            user_repo.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            pool.as_ref().clone(),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        // Use the same real repository for admin routes
        let admin_user_repo: Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait> = user_repo.clone();

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["username"], "newusername");
        assert_eq!(body["data"]["id"], fixture.test_target_user_id.to_string());
        
        // Cleanup after test
        db_helper.cleanup_all().await.expect("Failed to cleanup database");
    }

    #[actix_rt::test]
    async fn test_update_user_username_empty() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateUsernameRequest {
            username: "".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("cannot be empty"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_whitespace_only() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateUsernameRequest {
            username: "   ".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("cannot be empty"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_self_edit_forbidden() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateUsernameRequest {
            username: "newadminname".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_not_found() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        let non_existent_id = Uuid::new_v4();
        
        let update_data = UpdateUsernameRequest {
            username: "newusername".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", non_existent_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_forbidden_as_user() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateUsernameRequest {
            username: "hackerusername".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}

#[cfg(test)]
mod admin_user_status_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_user_status_activate_success() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateStatusRequest {
            is_active: true,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["is_active"], true);
        assert_eq!(body["data"]["id"], fixture.test_target_user_id.to_string());
    }

    #[actix_rt::test]
    async fn test_update_user_status_deactivate_success() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["is_active"], false);
        assert_eq!(body["data"]["id"], fixture.test_target_user_id.to_string());
    }

    #[actix_rt::test]
    async fn test_update_user_status_self_edit_forbidden() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_not_found() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        let non_existent_id = Uuid::new_v4();
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", non_existent_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_forbidden_as_user() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}

#[cfg(test)]
mod admin_user_delete_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_delete_user_success() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert!(body["message"].as_str().unwrap().contains("deleted successfully"));
    }

    #[actix_rt::test]
    async fn test_delete_user_self_delete_forbidden() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", fixture.test_admin_id), &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("cannot delete your own account"));
    }

    #[actix_rt::test]
    async fn test_delete_user_not_found() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        let non_existent_id = Uuid::new_v4();
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", non_existent_id), &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        // ApiError responses have "message" and "error_type" fields, not "success"
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_delete_user_forbidden_as_user() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_user_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_delete_user_unauthorized_without_token() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = test::TestRequest::delete()
            .uri(&format!("/api/admin/users/{}", fixture.test_target_user_id))
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn test_delete_user_invalid_uuid() {
        let fixture = test_common::UnifiedTestFixture::new_with_database().await;
        
        let req = create_auth_request("DELETE", "/api/admin/users/invalid-uuid", &fixture.test_admin_token)
            .to_request();

        // Create standard mock services
        let (auth_service, user_handler, admin_user_repo) = test_common::create_standard_mock_services_with_repo(fixture.test_user_id, fixture.test_admin_id, fixture.test_target_user_id).await;

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(Arc::new(test_common::mocks::create_mock_token_revocation_service()) as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(test_common::mocks::create_mock_active_token_service()))
                .app_data(web::Data::new(admin_user_repo))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        // Actix-web returns 404 for invalid UUID in path parameter, not 400
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}

#[cfg(test)]
mod enhanced_integration_tests {
    use super::*;

    /// ENHANCED PATTERN DEMO: Get user with integration test utilities
    #[actix_rt::test]
    async fn test_get_user_enhanced_integration() {
        // Use enhanced TestConfig for integration testing
        let config = EnhancedTestConfig::new_for_integration_tests().await;
        
        // Seed test data using enhanced utilities
        let admin_data = seed_admin_test_data(config.pool.as_ref()).await;
        
        // Create request using admin token
        let admin_token = generate_test_token(admin_data.0.id, "admin", 3600)
            .expect("Failed to generate admin token");
        
        let req = create_auth_request(
            "GET",
            &format!("/api/admin/users/{}", admin_data.1.id),
            &admin_token
        ).to_request();

        // Set up real services with database
        let admin_auth = HttpAuthentication::bearer(admin_validator);
        
        let user_repo = Arc::new(oxidizedoasis_websands::core::user::repository::UserRepository::new((*config.pool).clone()));
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        let auth_service = Arc::new(AuthService::new(
            user_repo.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            config.pool.as_ref().clone(),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(config.config.clone()))
                .app_data(web::Data::new(token_revocation_service as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .app_data(web::Data::new(user_repo as Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait>))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        
        let resp = test::call_service(&app, req).await;
        
        // Use enhanced assertion helper
        // Use enhanced assertion helper
        let body: Value = test::read_body_json(resp).await;
        assert_user_response(&body, "target-user@test.com", "user");
        
        // Verify user exists in database using enhanced utility
        assert_user_in_database(config.pool.as_ref(), admin_data.1.id, "target-user@test.com").await;
    }

    /// ENHANCED PATTERN DEMO: Update user role with scenario builder
    #[actix_rt::test]
    async fn test_update_user_role_with_scenario() {
        // Use scenario builder for complex workflow testing
        let scenario = UserManagementScenario::new()
            .await
            .with_admin_user()
            .await
            .with_target_users(1)
            .await;
            
        // Use the same config from the scenario
        let config = &scenario.config;
            
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let admin_token = generate_test_token(scenario.admin().id, "admin", 3600)
            .expect("Failed to generate admin token");
        
        let req = create_auth_request(
            "PUT",
            &format!("/api/admin/users/{}/role", scenario.target_user(0).id),
            &admin_token
        )
        .set_json(&update_data)
        .to_request();

        // Set up real services with database
        let admin_auth = HttpAuthentication::bearer(admin_validator);
        
        let user_repo = Arc::new(oxidizedoasis_websands::core::user::repository::UserRepository::new((*config.pool).clone()));
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        let auth_service = Arc::new(AuthService::new(
            user_repo.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            config.pool.as_ref().clone(),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(config.config.clone()))
                .app_data(web::Data::new(token_revocation_service as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .app_data(web::Data::new(user_repo as Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait>))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["role"], "admin");
        assert_eq!(body["data"]["id"], scenario.target_user(0).id.to_string());
        
        // Verify role update in database
        assert_user_in_database(config.pool.as_ref(), scenario.target_user(0).id, &scenario.target_user(0).email.clone().unwrap()).await;
    }

    /// ENHANCED PATTERN DEMO: Error handling with assertion helpers
    #[actix_rt::test]
    async fn test_user_not_found_enhanced_assertions() {
        let config = EnhancedTestConfig::new_for_integration_tests().await;
        let admin_data = seed_admin_test_data(config.pool.as_ref()).await;
        let non_existent_id = Uuid::new_v4();
        
        let admin_token = generate_test_token(admin_data.0.id, "admin", 3600)
            .expect("Failed to generate admin token");

        let req = create_auth_request(
            "GET",
            &format!("/api/admin/users/{}", non_existent_id),
            &admin_token
        ).to_request();

        // Set up real services with database
        let admin_auth = HttpAuthentication::bearer(admin_validator);
        
        let user_repo = Arc::new(oxidizedoasis_websands::core::user::repository::UserRepository::new((*config.pool).clone()));
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        let auth_service = Arc::new(AuthService::new(
            user_repo.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            config.pool.as_ref().clone(),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(config.config.clone()))
                .app_data(web::Data::new(token_revocation_service as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .app_data(web::Data::new(user_repo as Arc<dyn oxidizedoasis_websands::core::user::UserRepositoryTrait>))
                .service(
                    web::scope("/api/admin/users")
                        .wrap(admin_auth)
                        .route("", web::get().to(list_users))
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}/role", web::put().to(update_user_role))
                        .route("/{id}/username", web::put().to(update_user_username))
                        .route("/{id}/status", web::put().to(update_user_status))
                        .route("/{id}", web::delete().to(delete_user))
                )
        ).await;
        
        let resp = test::call_service(&app, req).await;
        
        // Use enhanced error assertion helper
        let body: Value = test::read_body_json(resp).await;
        assert_error_response(&body, "not found");
    }
}