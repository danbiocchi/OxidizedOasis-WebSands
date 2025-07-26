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

mod common;
use common::{
    create_test_app_config, create_test_user, generate_test_token,
    test_data::*, http::*, mocks::*
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

/// Test fixture for admin user management tests
struct AdminTestFixture {
    config: AppConfig,
    test_user_id: Uuid,
    test_admin_id: Uuid,
    test_target_user_id: Uuid,
    test_user_token: String,
    test_admin_token: String,
}

impl AdminTestFixture {
    async fn new() -> Self {
        let config = create_test_app_config();
        let test_user_id = Uuid::new_v4();
        let test_admin_id = Uuid::new_v4();
        let test_target_user_id = Uuid::new_v4();
        
        let test_user_token = generate_test_token(test_user_id, "user", 3600)
            .expect("Failed to generate user token");
        let test_admin_token = generate_test_token(test_admin_id, "admin", 3600)
            .expect("Failed to generate admin token");

        Self {
            config,
            test_user_id,
            test_admin_id,
            test_target_user_id,
            test_user_token,
            test_admin_token,
        }
    }

}

#[cfg(test)]
mod admin_user_list_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_list_users_success_as_admin() {
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/users", &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/users", &fixture.test_user_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = test::TestRequest::get()
            .uri("/api/admin/users")
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/users", "invalid.jwt.token")
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("GET", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
    }

    #[actix_rt::test]
    async fn test_get_user_not_found() {
        let fixture = AdminTestFixture::new().await;
        let non_existent_id = Uuid::new_v4();
        
        let req = create_auth_request("GET", &format!("/api/admin/users/{}", non_existent_id), &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_get_user_forbidden_as_user() {
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("GET", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_user_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/users/invalid-uuid", &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
    }
}

#[cfg(test)]
mod admin_user_role_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_user_role_success() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateRoleRequest {
            role: "invalid_role".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("Invalid role"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_self_edit_forbidden() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateRoleRequest {
            role: "user".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_not_found() {
        let fixture = AdminTestFixture::new().await;
        let non_existent_id = Uuid::new_v4();
        
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", non_existent_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_forbidden_as_user() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateRoleRequest {
            role: "admin".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        // Test both valid roles
        for role in &["user", "admin"] {
            let update_data = UpdateRoleRequest {
                role: role.to_string(),
            };

            let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
                .set_json(&update_data)
                .to_request();

            // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateUsernameRequest {
            username: "newusername".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
    }

    #[actix_rt::test]
    async fn test_update_user_username_empty() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateUsernameRequest {
            username: "".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("cannot be empty"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_whitespace_only() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateUsernameRequest {
            username: "   ".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("cannot be empty"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_self_edit_forbidden() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateUsernameRequest {
            username: "newadminname".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_not_found() {
        let fixture = AdminTestFixture::new().await;
        let non_existent_id = Uuid::new_v4();
        
        let update_data = UpdateUsernameRequest {
            username: "newusername".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", non_existent_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_user_username_forbidden_as_user() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateUsernameRequest {
            username: "hackerusername".to_string(),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_target_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateStatusRequest {
            is_active: true,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_not_found() {
        let fixture = AdminTestFixture::new().await;
        let non_existent_id = Uuid::new_v4();
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", non_existent_id), &fixture.test_admin_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_forbidden_as_user() {
        let fixture = AdminTestFixture::new().await;
        
        let update_data = UpdateStatusRequest {
            is_active: false,
        };

        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_user_token)
            .set_json(&update_data)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", fixture.test_admin_id), &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("cannot delete your own account"));
    }

    #[actix_rt::test]
    async fn test_delete_user_not_found() {
        let fixture = AdminTestFixture::new().await;
        let non_existent_id = Uuid::new_v4();
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", non_existent_id), &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        assert_eq!(body["success"], false);
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_delete_user_forbidden_as_user() {
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("DELETE", &format!("/api/admin/users/{}", fixture.test_target_user_id), &fixture.test_user_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = test::TestRequest::delete()
            .uri(&format!("/api/admin/users/{}", fixture.test_target_user_id))
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
        let fixture = AdminTestFixture::new().await;
        
        let req = create_auth_request("DELETE", "/api/admin/users/invalid-uuid", &fixture.test_admin_token)
            .to_request();

        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_user = create_test_user(fixture.test_user_id, TEST_USER_USERNAME, TEST_USER_EMAIL, true, "user");
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_target_user = create_test_user(fixture.test_target_user_id, "targetuser", "target@example.com", true, "user");
        
        let test_user_id = fixture.test_user_id;
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        // Clone objects for different closures to avoid ownership issues
        let test_user_for_find_by_id = test_user.clone();
        let test_admin_for_find_by_id = test_admin.clone();
        let test_target_user_for_find_by_id = test_target_user.clone();
        
        let test_user_for_find_all = test_user.clone();
        let test_admin_for_find_all = test_admin.clone();
        let test_target_user_for_find_all = test_target_user.clone();
        
        let test_target_user_for_update_role = test_target_user.clone();
        let test_target_user_for_update_username = test_target_user.clone();
        let test_target_user_for_update_status = test_target_user.clone();
        
        // Mock find_by_id for authentication and operations
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_user_id {
                    Ok(Some(test_user_for_find_by_id.clone()))
                } else if id == test_admin_id {
                    Ok(Some(test_admin_for_find_by_id.clone()))
                } else if id == test_target_user_id {
                    Ok(Some(test_target_user_for_find_by_id.clone()))
                } else {
                    Ok(None)
                }
            });

        // Mock find_all for listing users
        let all_users = vec![test_user_for_find_all.clone(), test_admin_for_find_all.clone(), test_target_user_for_find_all.clone()];
        user_repo.expect_find_all()
            .returning(move || Ok(all_users.clone()));

        // Mock update operations
        user_repo.expect_update_role()
            .returning(move |id, role| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_role.clone();
                    updated_user.role = role.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_username()
            .returning(move |id, username| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_username.clone();
                    updated_user.username = username.to_string();
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_update_status()
            .returning(move |id, is_active| {
                if id == test_target_user_id {
                    let mut updated_user = test_target_user_for_update_status.clone();
                    updated_user.is_active = is_active;
                    Ok(Some(updated_user))
                } else {
                    Ok(None)
                }
            });

        user_repo.expect_delete()
            .returning(move |id| {
                if id == test_target_user_id {
                    Ok(true)
                } else {
                    Ok(false)
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let mut app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
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
    }
}