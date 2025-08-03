//! User CRUD integration tests with proper authentication middleware setup

use actix_web::{test, web, App, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

use oxidizedoasis_websands::core::user::{User, UserRepositoryTrait, UserError};
use oxidizedoasis_websands::core::user::repository::MockUserRepositoryTrait;
use oxidizedoasis_websands::api::routes::admin::user_management::{
    update_user_role, update_user_status, UpdateRoleRequest, UpdateStatusRequest,
    list_users, get_user, delete_user, update_user_username, UpdateUsernameRequest
};
use oxidizedoasis_websands::core::auth::{AuthService};
use oxidizedoasis_websands::core::auth::active_token::ActiveTokenService;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService;
use oxidizedoasis_websands::core::email::service::EmailService;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::infrastructure::middleware::admin_validator;
use actix_web_httpauth::middleware::HttpAuthentication;
use oxidizedoasis_websands::common::error::ApiErrorType;

use test_common::{
    UnifiedTestFixture, create_test_user, create_standard_mock_services,
    test_data::*, http::*, mocks::*
};

// Helper function to create a mock user
fn mock_user(id: Uuid, username: &str, role: &str, is_active: bool) -> User {
    User {
        id,
        username: username.to_string(),
        email: Some(format!("{}@example.com", username)),
        password_hash: "hashed_password".to_string(),
        role: role.to_string(),
        is_active,
        is_email_verified: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        verification_token: None,
        verification_token_expires_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_user_role_self_edit_forbidden() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_admin_id = fixture.test_admin_id;
        
        // Mock repo expectations (not strictly needed for this test as it should fail before DB ops)
        user_repo.expect_update_role().times(0); // Ensure no DB call is made
        
        // Mock find_by_id for authentication
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_admin_id {
                    Ok(Some(test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let user_repo_arc = Arc::new(user_repo);
        
        let auth_service = Arc::new(AuthService::new(
            user_repo_arc.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_repo_arc.clone() as Arc<dyn UserRepositoryTrait>))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/users")
                                .route("/{id}/role", web::put().to(update_user_role))
                        )
                )
        ).await;

        let req_payload = UpdateRoleRequest { role: "admin".to_string() };
        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error_type"], json!("Authorization"));
        assert!(body["message"].as_str().unwrap().contains("You cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_role_other_user_success() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        let target_user = mock_user(fixture.test_target_user_id, "target_user", "user", true);
        
        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        let updated_target_user = User { role: "admin".to_string(), ..target_user.clone() };

        user_repo.expect_update_role()
            .withf(move |id, role| *id == test_target_user_id && role == "admin")
            .times(1)
            .returning(move |_, _| Ok(Some(updated_target_user.clone())));
        
        // Mock find_by_id for authentication
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_admin_id {
                    Ok(Some(test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let user_repo_arc = Arc::new(user_repo);
        
        let auth_service = Arc::new(AuthService::new(
            user_repo_arc.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_repo_arc.clone() as Arc<dyn UserRepositoryTrait>))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/users")
                                .route("/{id}/role", web::put().to(update_user_role))
                        )
                )
        ).await;

        let req_payload = UpdateRoleRequest { role: "admin".to_string() };
        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/role", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(true));
        assert_eq!(body["data"]["role"], json!("admin"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_self_edit_forbidden() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_admin_id = fixture.test_admin_id;

        user_repo.expect_update_status().times(0);
        
        // Mock find_by_id for authentication
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_admin_id {
                    Ok(Some(test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let user_repo_arc = Arc::new(user_repo);
        
        let auth_service = Arc::new(AuthService::new(
            user_repo_arc.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_repo_arc.clone() as Arc<dyn UserRepositoryTrait>))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/users")
                                .route("/{id}/status", web::put().to(update_user_status))
                        )
                )
        ).await;

        let req_payload = UpdateStatusRequest { is_active: false };
        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error_type"], json!("Authorization"));
        assert!(body["message"].as_str().unwrap().contains("You cannot edit your own account"));
    }

    #[actix_rt::test]
    async fn test_update_user_status_other_user_success() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        let target_user = mock_user(fixture.test_target_user_id, "target_user", "user", true);
        
        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_admin_id = fixture.test_admin_id;
        let test_target_user_id = fixture.test_target_user_id;
        
        let updated_target_user = User { is_active: false, ..target_user.clone() };

        user_repo.expect_update_status()
            .withf(move |id, is_active| *id == test_target_user_id && !*is_active)
            .times(1)
            .returning(move |_, _| Ok(Some(updated_target_user.clone())));
        
        // Mock find_by_id for authentication
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_admin_id {
                    Ok(Some(test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let user_repo_arc = Arc::new(user_repo);
        
        let auth_service = Arc::new(AuthService::new(
            user_repo_arc.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_repo_arc.clone() as Arc<dyn UserRepositoryTrait>))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/users")
                                .route("/{id}/status", web::put().to(update_user_status))
                        )
                )
        ).await;

        let req_payload = UpdateStatusRequest { is_active: false };
        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["success"], json!(true));
        // The UserAdminView for the response doesn't directly include `is_active`.
        // We trust the handler uses the updated user from repo for its response.
        // If UserAdminView is updated to include is_active, this assertion can be more specific.
        // For now, checking success is sufficient as the repo mock ensures correct data was returned.
    }

    // Example test for update_user_username (to show self-edit is also blocked there by current code)
    // This is not strictly required by the subtask but confirms the pattern.
    #[actix_rt::test]
    async fn test_update_user_username_self_edit_forbidden() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_admin_id = fixture.test_admin_id;

        user_repo.expect_find_by_id().times(0); // Should fail before this
        user_repo.expect_update_username().times(0);
        
        // Mock find_by_id for authentication
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_admin_id {
                    Ok(Some(test_admin.clone()))
                } else {
                    Ok(None)
                }
            });

        let user_repo_arc = Arc::new(user_repo);
        
        let auth_service = Arc::new(AuthService::new(
            user_repo_arc.clone(),
            test_common::TEST_JWT_SECRET.to_string(),
            test_common::TEST_AUDIENCE.to_string(),
            token_revocation_service.clone(),
            active_token_service.clone(),
            email_service.clone(),
        ));

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_repo_arc.clone() as Arc<dyn UserRepositoryTrait>))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(auth_service))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/users")
                                .route("/{id}/username", web::put().to(update_user_username))
                        )
                )
        ).await;

        let req_payload = UpdateUsernameRequest { username: "new_admin_name".to_string() };
        let req = create_auth_request("PUT", &format!("/api/admin/users/{}/username", fixture.test_admin_id), &fixture.test_admin_token)
            .set_json(&req_payload)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["error_type"], json!("Authorization"));
        assert!(body["message"].as_str().unwrap().contains("You cannot edit your own account"));
    }
}
