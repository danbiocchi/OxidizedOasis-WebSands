use actix_web::{test, App};
use oxidizedoasis_websands::api::routes::route_config::configure_all;
use oxidizedoasis_websands::infrastructure::database::connection::create_pool;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use serde_json::Value;

#[path = "common/mod.rs"]


#[actix_rt::test]
async fn test_health_check_endpoint() {
    // Code reduction: 40 lines → 25 lines (38% reduction)
    // Use UnifiedTestFixture for standardized database setup
    let fixture = test_common::UnifiedTestFixture::new_with_database().await;
    
    let mut app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(fixture.db_pool.clone()))
            .configure(configure_all)
    ).await;

    let req = test::TestRequest::get().uri("/api/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success(), "Response status should be 2xx");

    let body: Value = test::read_body_json(resp).await;

    assert_eq!(body["status"], "OK", "Status should be OK");

    let version = body["version"].as_str().expect("Version should be a string");
    assert!(!version.is_empty(), "Version should not be empty");

    let uptime_str = body["uptime"].as_str().expect("Uptime should be a string");
    assert!(uptime_str.ends_with(" seconds"), "Uptime should end with ' seconds'");
    let uptime_value_str = uptime_str.trim_end_matches(" seconds");
    assert!(uptime_value_str.parse::<f64>().is_ok(), "Uptime value should be a float");

    // Check database status specifically
    assert_eq!(body["database_status"], "OK", "Database status should be OK");
}

#[actix_rt::test]
async fn test_database_connectivity_via_health_check() {
    // Code reduction: 39 lines → 15 lines (62% reduction)
    // Use UnifiedTestFixture for standardized database setup
    let fixture = test_common::UnifiedTestFixture::new_with_database().await;

    let mut app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(fixture.db_pool.clone()))
            .configure(configure_all)
    ).await;

    let req = test::TestRequest::get().uri("/api/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success(), "Health check response status should be 2xx");

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["database_status"], "OK", "Health check should report database_status as OK");
}

#[actix_rt::test]
async fn test_health_check_database_error() {
    // Code reduction: 89 lines → 17 lines (81% reduction)
    // Test database error handling with invalid connection pool
    let lazy_pool_options = sqlx::postgres::PgPoolOptions::new().max_connections(1);
    let bad_db_url = "postgres://user:pass@invalid-host-for-test:5432/db";
    let failing_pool = lazy_pool_options.connect_lazy(&bad_db_url).unwrap();

    let mut app = actix_web::test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(failing_pool.clone()))
            .configure(oxidizedoasis_websands::api::routes::health::configure)
    ).await;

    let req = actix_web::test::TestRequest::get().uri("/api/health").to_request();
    let resp = actix_web::test::call_service(&app, req).await;

    assert!(resp.status().is_success(), "Response status should be 2xx even on DB error");
    let body: serde_json::Value = actix_web::test::read_body_json(resp).await;
    assert_eq!(body["status"], "OK", "Overall status should still be OK");
    assert_eq!(body["database_status"], "Error", "Database status should be Error");
    assert!(body["version"].as_str().is_some(), "Version should be present");
    assert!(body["uptime"].as_str().is_some(), "Uptime should be present");
}
