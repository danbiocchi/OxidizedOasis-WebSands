//! Comprehensive admin logs management integration tests
//! Tests all admin logs endpoints with various scenarios and edge cases

use actix_web::{test, web, App, http::StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use oxidizedoasis_websands::api::routes::admin::logs::{
    get_logs, get_log_settings, update_log_settings
};
use oxidizedoasis_websands::core::user::UserRepositoryTrait;
use oxidizedoasis_websands::core::auth::{AuthService};
use oxidizedoasis_websands::core::auth::active_token::ActiveTokenService;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationService;
use oxidizedoasis_websands::core::email::service::EmailService;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::infrastructure::middleware::admin_validator;
use actix_web_httpauth::middleware::HttpAuthentication;

mod common;
use common::{
    create_test_config_with_cleanup, cleanup_test_database, create_test_user, generate_test_token,
    test_data::*, http::*, env::with_env_vars, mocks::*
};

/// Test structure for updating log settings
#[derive(serde::Serialize)]
struct UpdateLogSettingsRequest {
    retention_days: Option<i32>,
    min_level: Option<String>,
    enabled_sources: Option<Vec<String>>,
}

/// Test fixture for admin logs tests
struct AdminLogsTestFixture {
    config: AppConfig,
    db_name: String,
    test_user_id: Uuid,
    test_admin_id: Uuid,
    test_user_token: String,
    test_admin_token: String,
}

impl AdminLogsTestFixture {
    async fn new() -> Self {
        let (config, db_name) = create_test_config_with_cleanup().await
            .expect("Failed to create test config with cleanup");
        let test_user_id = Uuid::new_v4();
        let test_admin_id = Uuid::new_v4();
        
        let test_user_token = generate_test_token(test_user_id, "user", 3600)
            .expect("Failed to generate user token");
        let test_admin_token = generate_test_token(test_admin_id, "admin", 3600)
            .expect("Failed to generate admin token");

        Self {
            config,
            db_name,
            test_user_id,
            test_admin_id,
            test_user_token,
            test_admin_token,
        }
    }

    async fn cleanup(&self) {
        if let Err(e) = cleanup_test_database(&self.db_name).await {
            eprintln!("Failed to cleanup test database {}: {}", self.db_name, e);
        }
    }
}

#[cfg(test)]
mod admin_logs_retrieval_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_get_logs_success_as_admin() {
        let fixture = AdminLogsTestFixture::new().await;
        
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
        
        // Mock find_by_id for authentication
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

        let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
            .unwrap_or_else(|_| panic!("Could not create database pool"));
        let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
            pool,
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
                .service(
                    web::scope("/api/admin")
                        .wrap(admin_auth)
                        .service(
                            web::scope("/logs")
                                .route("", web::get().to(get_logs))
                                .route("/settings", web::get().to(get_log_settings))
                                .route("/settings", web::put().to(update_log_settings))
                        )
                )
        ).await;
        
        let req = create_auth_request("GET", "/api/admin/logs", &fixture.test_admin_token)
            .to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        
        // Debug output to understand the failure
        if status != StatusCode::OK {
            let body = test::read_body(resp).await;
            println!("❌ Admin logs test failed with status: {:?}", status);
            println!("❌ Response body: {:?}", String::from_utf8_lossy(&body));
            
            // Also print token info for debugging
            println!("🔍 Generated admin token (first 50 chars): {}",
                     &fixture.test_admin_token.chars().take(50).collect::<String>());
            println!("🔍 Test admin ID: {}", fixture.test_admin_id);
            
            // Assert here to fail the test with debug output
            assert_eq!(status, StatusCode::OK);
            return; // This won't be reached but prevents further execution
        }

        let body: Value = test::read_body_json(resp).await;
        assert!(body["logs"].is_array());
        assert!(body["total"].is_number());
        assert!(body["page"].is_number());
        assert!(body["per_page"].is_number());
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_with_pagination() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs?page=2&per_page=25", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["page"], 2);
        assert_eq!(body["per_page"], 25);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_with_level_filter() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs?level=error", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["logs"].is_array());
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_with_source_filter() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs?source=system", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["logs"].is_array());
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_with_date_range() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let start_date = "2024-01-01T00:00:00Z";
        let end_date = "2024-12-31T23:59:59Z";
        let req = create_auth_request("GET", &format!("/api/admin/logs?start_date={}&end_date={}", start_date, end_date), &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["logs"].is_array());
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_with_search_term() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs?search=login", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["logs"].is_array());
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_per_page_limit() {
        let fixture = AdminLogsTestFixture::new().await;
        
        // Test per_page limit is enforced (max 100)
        let req = create_auth_request("GET", "/api/admin/logs?per_page=150", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["per_page"], 100); // Should be capped at 100
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_forbidden_as_user() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs", &fixture.test_user_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_unauthorized_without_token() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = test::TestRequest::get()
            .uri("/api/admin/logs")
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_logs_invalid_token() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs", "invalid.jwt.token")
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        
        fixture.cleanup().await;
    }
}

#[cfg(test)]
mod admin_log_settings_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_get_log_settings_success() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["retention_days"].is_number());
        assert!(body["min_level"].is_string());
        assert!(body["enabled_sources"].is_array());
        
        // Check default values
        assert_eq!(body["retention_days"], 30);
        assert_eq!(body["min_level"], "info");
        let sources = body["enabled_sources"].as_array().unwrap();
        assert!(sources.contains(&json!("system")));
        assert!(sources.contains(&json!("auth")));
        assert!(sources.contains(&json!("api")));
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_log_settings_forbidden_as_user() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = create_auth_request("GET", "/api/admin/logs/settings", &fixture.test_user_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_get_log_settings_unauthorized_without_token() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = test::TestRequest::get()
            .uri("/api/admin/logs/settings")
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        
        fixture.cleanup().await;
    }
}

#[cfg(test)]
mod admin_log_settings_update_tests {
    use super::*;

    #[actix_rt::test]
    async fn test_update_log_settings_success_partial() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: Some(60),
            min_level: None,
            enabled_sources: None,
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["retention_days"], 60);
        assert_eq!(body["min_level"], "info"); // Should use default
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_success_complete() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: Some(90),
            min_level: Some("debug".to_string()),
            enabled_sources: Some(vec!["system".to_string(), "auth".to_string()]),
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        assert_eq!(body["retention_days"], 90);
        assert_eq!(body["min_level"], "debug");
        let sources = body["enabled_sources"].as_array().unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&json!("system")));
        assert!(sources.contains(&json!("auth")));
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_invalid_retention_days_too_low() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: Some(0),
            min_level: None,
            enabled_sources: None,
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("between 1 and 365"));
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_invalid_retention_days_too_high() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: Some(400),
            min_level: None,
            enabled_sources: None,
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("between 1 and 365"));
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_invalid_log_level() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: None,
            min_level: Some("invalid".to_string()),
            enabled_sources: None,
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body: Value = test::read_body_json(resp).await;
        assert!(body["message"].as_str().unwrap().contains("Invalid log level"));
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_valid_log_levels() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let valid_levels = vec!["error", "warn", "info", "debug", "trace"];
        
        for level in valid_levels {
            let update_data = UpdateLogSettingsRequest {
                retention_days: None,
                min_level: Some(level.to_string()),
                enabled_sources: None,
            };

            let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["min_level"], level);
        }
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_edge_case_retention_days() {
        let fixture = AdminLogsTestFixture::new().await;
        
        // Test boundary values
        for days in &[1, 365] {
            let update_data = UpdateLogSettingsRequest {
                retention_days: Some(*days),
                min_level: None,
                enabled_sources: None,
            };

            let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);

            let body: Value = test::read_body_json(resp).await;
            assert_eq!(body["retention_days"], *days);
        }
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_empty_enabled_sources() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: None,
            min_level: None,
            enabled_sources: Some(vec![]),
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_admin_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let body: Value = test::read_body_json(resp).await;
        let sources = body["enabled_sources"].as_array().unwrap();
        assert_eq!(sources.len(), 0);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_forbidden_as_user() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: Some(60),
            min_level: None,
            enabled_sources: None,
        };

        let req = create_auth_request("PUT", "/api/admin/logs/settings", &fixture.test_user_token)
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_unauthorized_without_token() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let update_data = UpdateLogSettingsRequest {
            retention_days: Some(60),
            min_level: None,
            enabled_sources: None,
        };

        let req = test::TestRequest::put()
            .uri("/api/admin/logs/settings")
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        
        fixture.cleanup().await;
    }

    #[actix_rt::test]
    async fn test_update_log_settings_malformed_json() {
        let fixture = AdminLogsTestFixture::new().await;
        
        let req = test::TestRequest::put()
            .uri("/api/admin/logs/settings")
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

            let pool = oxidizedoasis_websands::infrastructure::database::connection::create_pool(&fixture.config).await
                .unwrap_or_else(|_| panic!("Could not create database pool"));
            let user_handler = oxidizedoasis_websands::api::handlers::user_handler::create_handler(
                pool,
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
                    .service(
                        web::scope("/api/admin")
                            .wrap(admin_auth)
                            .service(
                                web::scope("/logs")
                                    .route("", web::get().to(get_logs))
                                    .route("/settings", web::get().to(get_log_settings))
                                    .route("/settings", web::put().to(update_log_settings))
                            )
                    )
            ).await;
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        
        fixture.cleanup().await;
    }
}