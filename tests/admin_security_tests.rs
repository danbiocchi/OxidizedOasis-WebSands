//! Comprehensive admin security incidents integration tests
//! Tests all admin security endpoints with various scenarios and edge cases

use actix_web::{test, web, App, http::StatusCode};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

use oxidizedoasis_websands::api::routes::admin::security::{
    list_incidents, create_incident, get_incident, update_incident_status
};
use oxidizedoasis_websands::infrastructure::middleware::admin_validator;
use actix_web_httpauth::middleware::HttpAuthentication;

mod common;
use common::{
    UnifiedTestFixture, create_test_user,
    test_data::*, http::*, mocks::*
};
use oxidizedoasis_websands::core::auth::service::AuthService;

/// Test structure for creating security incidents
#[derive(serde::Serialize)]
struct CreateIncidentRequest {
    title: String,
    description: String,
    severity: String,
    assigned_to: Option<Uuid>,
}

/// Test structure for updating incident status
#[derive(serde::Serialize)]
struct UpdateIncidentStatusRequest {
    status: String,
    resolution_notes: Option<String>,
}

#[cfg(test)]
mod admin_security_incidents_list_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_list_incidents_success_as_admin() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;

        let (auth_service, user_handler, _token_revocation_service) = common::create_standard_mock_services(
            fixture.test_user_id,
            fixture.test_admin_id
        ).await;

        // Create individual services for app_data
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service.clone()))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents", &fixture.test_admin_token)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["incidents"].is_array());
        assert!(body["total"].is_number());
        assert!(body["page"].is_number());
        assert!(body["per_page"].is_number());
    }

    #[actix_rt::test]
    async fn test_list_incidents_with_pagination() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Create mock services
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
        let test_admin = create_test_user(fixture.test_admin_id, TEST_ADMIN_USERNAME, TEST_ADMIN_EMAIL, true, "admin");
        let test_admin_id = fixture.test_admin_id;
        
        user_repo.expect_find_by_id()
            .returning(move |id| {
                if id == test_admin_id {
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

        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|e| panic!("Failed to create database pool: {}", e)),
            email_service.clone(),
            auth_service,
            token_revocation_service.clone(),
            active_token_service.clone(),
        );

        let admin_auth = HttpAuthentication::bearer(admin_validator);

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(user_handler))
                .app_data(web::Data::new(fixture.config.clone()))
                .app_data(web::Data::new(token_revocation_service.clone() as Arc<dyn oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait>))
                .app_data(web::Data::new(active_token_service))
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents?page=2&per_page=10", &fixture.test_admin_token)
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["page"], 2);
        assert_eq!(body["per_page"], 10);
    }

    #[actix_rt::test]
    async fn test_list_incidents_with_severity_filter() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents?severity=critical", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["incidents"].is_array());
    }

    #[actix_rt::test]
    async fn test_list_incidents_with_status_filter() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents?status=open", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["incidents"].is_array());
    }

    #[actix_rt::test]
    async fn test_list_incidents_with_date_range() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let start_date = "2024-01-01T00:00:00Z";
        let end_date = "2024-12-31T23:59:59Z";
        let req = create_auth_request("GET", &format!("/api/admin/security/incidents?start_date={}&end_date={}", start_date, end_date), &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["incidents"].is_array());
    }

    #[actix_rt::test]
    async fn test_list_incidents_per_page_limit() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Test per_page limit is enforced (max 100)
        let req = create_auth_request("GET", "/api/admin/security/incidents?per_page=150", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["per_page"], 100); // Should be capped at 100
    }

    #[actix_rt::test]
    async fn test_list_incidents_forbidden_as_user() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents", &fixture.test_user_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_list_incidents_unauthorized_without_token() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::get()
            .uri("/api/admin/security/incidents")
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn test_list_incidents_invalid_token() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents", "invalid.jwt.token")
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

#[cfg(test)]
mod admin_security_incidents_create_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_create_incident_success() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let incident_data = CreateIncidentRequest {
            title: "Security Breach Detected".to_string(),
            description: "Suspicious activity detected in user authentication logs".to_string(),
            severity: "high".to_string(),
            assigned_to: Some(fixture.test_admin_id),
        };

        let req = create_auth_request("POST", "/api/admin/security/incidents", &fixture.test_admin_token)
            .set_json(&incident_data)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["id"].is_string());
        assert_eq!(body["title"], "Security Breach Detected");
        assert_eq!(body["description"], "Suspicious activity detected in user authentication logs");
        assert_eq!(body["severity"], "high");
        assert_eq!(body["status"], "open");
        assert_eq!(body["reported_by"], fixture.test_admin_id.to_string());
        assert_eq!(body["assigned_to"], fixture.test_admin_id.to_string());
    }

    #[actix_rt::test]
    async fn test_create_incident_success_without_assignment() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let incident_data = CreateIncidentRequest {
            title: "Minor Security Issue".to_string(),
            description: "Low priority security concern".to_string(),
            severity: "low".to_string(),
            assigned_to: None,
        };

        let req = create_auth_request("POST", "/api/admin/security/incidents", &fixture.test_admin_token)
            .set_json(&incident_data)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["id"].is_string());
        assert_eq!(body["title"], "Minor Security Issue");
        assert_eq!(body["severity"], "low");
        assert_eq!(body["status"], "open");
        assert!(body["assigned_to"].is_null());
    }

    #[actix_rt::test]
    async fn test_create_incident_all_severity_levels() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let severities = vec!["low", "medium", "high", "critical"];
        
        for severity in severities {
            let incident_data = CreateIncidentRequest {
                title: format!("{} severity incident", severity),
                description: format!("Test incident with {} severity", severity),
                severity: severity.to_string(),
                assigned_to: None,
            };

            let req = create_auth_request("POST", "/api/admin/security/incidents", &fixture.test_admin_token)
                .set_json(&incident_data)
                .to_request();

            // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::CREATED);

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["severity"], severity);
        }
    }

    #[actix_rt::test]
    async fn test_create_incident_empty_title() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let incident_data = CreateIncidentRequest {
            title: "".to_string(),
            description: "Test description".to_string(),
            severity: "medium".to_string(),
            assigned_to: None,
        };

        let req = create_auth_request("POST", "/api/admin/security/incidents", &fixture.test_admin_token)
            .set_json(&incident_data)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("Title is required"));
    }

    #[actix_rt::test]
    async fn test_create_incident_empty_description() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let incident_data = CreateIncidentRequest {
            title: "Test Title".to_string(),
            description: "".to_string(),
            severity: "medium".to_string(),
            assigned_to: None,
        };

        let req = create_auth_request("POST", "/api/admin/security/incidents", &fixture.test_admin_token)
            .set_json(&incident_data)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("Description is required"));
    }

    #[actix_rt::test]
    async fn test_create_incident_forbidden_as_user() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let incident_data = CreateIncidentRequest {
            title: "User Incident".to_string(),
            description: "This should fail".to_string(),
            severity: "low".to_string(),
            assigned_to: None,
        };

        let req = create_auth_request("POST", "/api/admin/security/incidents", &fixture.test_user_token)
            .set_json(&incident_data)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_create_incident_unauthorized_without_token() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let incident_data = CreateIncidentRequest {
            title: "Unauthorized Incident".to_string(),
            description: "This should fail".to_string(),
            severity: "low".to_string(),
            assigned_to: None,
        };

        let req = test::TestRequest::post()
            .uri("/api/admin/security/incidents")
            .set_json(&incident_data)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn test_create_incident_malformed_json() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::post()
            .uri("/api/admin/security/incidents")
            .insert_header(("Authorization", format!("Bearer {}", fixture.test_admin_token)))
            .set_payload("{invalid json")
            .insert_header(("content-type", "application/json"))
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}

#[cfg(test)]
mod admin_security_incidents_detail_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_get_incident_not_found() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", &format!("/api/admin/security/incidents/{}", fixture.test_target_user_id), &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_get_incident_forbidden_as_user() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", &format!("/api/admin/security/incidents/{}", fixture.test_target_user_id), &fixture.test_user_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_get_incident_unauthorized_without_token() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::get()
            .uri(&format!("/api/admin/security/incidents/{}", fixture.test_target_user_id))
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn test_get_incident_invalid_uuid() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/admin/security/incidents/invalid-uuid", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}

#[cfg(test)]
mod admin_security_incidents_update_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_incident_status_not_found() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let update_data = UpdateIncidentStatusRequest {
            status: "resolved".to_string(),
            resolution_notes: Some("Issue resolved".to_string()),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/security/incidents/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("not found"));
    }

    #[actix_rt::test]
    async fn test_update_incident_status_valid_statuses() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let valid_statuses = vec!["open", "in_progress", "resolved", "closed"];
        
        for status in valid_statuses {
            let update_data = UpdateIncidentStatusRequest {
                status: status.to_string(),
                resolution_notes: if status == "resolved" || status == "closed" {
                    Some("Test resolution".to_string())
                } else {
                    None
                },
            };

            let req = create_auth_request("PUT", &format!("/api/admin/security/incidents/{}/status", fixture.test_target_user_id), &fixture.test_admin_token)
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
            let resp = test::call_service(&app, req).await;
            // Expect 404 since incident doesn't exist, but validates the request structure
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        }
    }

    #[actix_rt::test]
    async fn test_update_incident_status_forbidden_as_user() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let update_data = UpdateIncidentStatusRequest {
            status: "resolved".to_string(),
            resolution_notes: Some("User trying to resolve".to_string()),
        };

        let req = create_auth_request("PUT", &format!("/api/admin/security/incidents/{}/status", fixture.test_target_user_id), &fixture.test_user_token)
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_update_incident_status_unauthorized_without_token() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let update_data = UpdateIncidentStatusRequest {
            status: "resolved".to_string(),
            resolution_notes: Some("Unauthorized update".to_string()),
        };

        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/security/incidents/{}/status", fixture.test_target_user_id))
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[actix_rt::test]
    async fn test_update_incident_status_invalid_uuid() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let update_data = UpdateIncidentStatusRequest {
            status: "resolved".to_string(),
            resolution_notes: Some("Test resolution".to_string()),
        };

        let req = create_auth_request("PUT", "/api/admin/security/incidents/invalid-uuid/status", &fixture.test_admin_token)
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[actix_rt::test]
    async fn test_update_incident_status_malformed_json() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = test::TestRequest::put()
            .uri(&format!("/api/admin/security/incidents/{}/status", fixture.test_target_user_id))
            .insert_header(("Authorization", format!("Bearer {}", fixture.test_admin_token)))
            .set_payload("{invalid json")
            .insert_header(("content-type", "application/json"))
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}

#[cfg(test)]
mod admin_security_edge_case_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_security_endpoints_method_not_allowed() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Test PATCH on incidents list endpoint (should be METHOD_NOT_ALLOWED)
        let req = test::TestRequest::patch()
            .uri("/api/admin/security/incidents")
            .insert_header(("Authorization", format!("Bearer {}", fixture.test_admin_token)))
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                                .default_service(web::route().to(oxidizedoasis_websands::api::routes::admin::security::method_not_allowed))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[actix_rt::test]
    async fn test_security_endpoints_route_not_found() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        let req = create_auth_request("GET", "/api/admin/security/nonexistent", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_rt::test]
    async fn test_security_incidents_large_pagination() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Test with very large page number
        let req = create_auth_request("GET", "/api/admin/security/incidents?page=999999&per_page=1", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["page"], 999999);
        assert_eq!(body["per_page"], 1);
    }

    #[actix_rt::test]
    async fn test_security_incidents_zero_page() {
        let fixture = UnifiedTestFixture::new_with_mocks().await;
        
        // Test with page=0 (should default to 1)
        let req = create_auth_request("GET", "/api/admin/security/incidents?page=0", &fixture.test_admin_token)
            .to_request();

        // Create mock services  
        let mut user_repo = create_mock_user_repository();
        let email_service = Arc::new(create_mock_email_service());
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        let active_token_service = Arc::new(create_mock_active_token_service());
        
        // Set up user repository expectations
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
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/security")
                                .route("/incidents", web::get().to(list_incidents))
                                .route("/incidents", web::post().to(create_incident))
                                .route("/incidents/{id}", web::get().to(get_incident))
                                .route("/incidents/{id}/status", web::put().to(update_incident_status))
                        )
                )
        ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["page"], 1); // Should default to 1
    }
}