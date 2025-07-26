use actix_web::{test, App};
use oxidizedoasis_websands::api::routes::route_config::configure_all;
use oxidizedoasis_websands::infrastructure::database::connection::create_pool;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use serde_json::Value;

#[path = "common/mod.rs"]
mod common;

#[actix_rt::test]
async fn test_health_check_endpoint() {
    // Load .env.test file for test database URL and other configurations
    dotenv::from_filename(".env.test").ok();

    let (config, db_name) = common::create_test_config_with_cleanup().await
        .expect("Failed to create test config with cleanup");
    
    let db_pool = create_pool(&config).await.expect("Failed to create database pool for test");

    let mut app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(db_pool.clone()))
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
    // This implicitly tests database connectivity if the main health_check logic includes a ping
    assert_eq!(body["database_status"], "OK", "Database status should be OK");
    
    // Cleanup test database
    common::cleanup_test_database(&db_name).await
        .expect("Failed to cleanup test database");
}

#[actix_rt::test]
async fn test_database_connectivity_via_health_check() {
    // Load .env.test file for test environment
    dotenv::from_filename(".env.test").ok();

    let (config, db_name) = common::create_test_config_with_cleanup().await
        .expect("Failed to create test config with cleanup");
    
    let db_pool = create_pool(&config).await.expect("Failed to create database pool for test");

    // Test a simple query
    // let result = sqlx::query("SELECT 1 as id")
    //     .fetch_one(&db_pool)
    //     .await;
    // assert!(result.is_ok(), "Database query failed: {:?}", result.err());
    // You could also check the value if needed:
    // use sqlx::Row;
    // let row = result.unwrap();
    // assert_eq!(row.get::<i32, _>("id"), 1);

    // Now, ensure the health check reports the DB as OK
    let mut app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(db_pool.clone()))
            .configure(configure_all)
    ).await;

    let req = test::TestRequest::get().uri("/api/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success(), "Health check response status should be 2xx");

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["database_status"], "OK", "Health check should report database_status as OK");
    
    // Cleanup test database
    common::cleanup_test_database(&db_name).await
        .expect("Failed to cleanup test database");
}

#[actix_rt::test]
async fn test_health_check_database_error() {
    // This test is tricky because of sqlx prepare issues.
    // We want to test the path where the database connection fails.
    // Ideally, we'd pass a PgPool that's configured to fail.
    // However, `sqlx prepare` needs to pass for `health.rs` to compile,
    // which needs a valid DB connection at compile time.

    // For now, let's write the test structure.
    // It will likely fail to compile or run without a solution for `sqlx prepare`.

    // Simulate an environment where the database is expected to be down.
    // We'll use the real app configuration which points to the default DB,
    // but we expect health_check to report an error if the DB isn't actually available.
    // This test fundamentally relies on the DB being down when `health_check` is called.
    // This is hard to guarantee in a test environment without external setup.

    // Let's assume the default pool is used, and we're in an environment where it can't connect.
    // The `health_check` function itself will try to acquire a connection.
    // If the DB configured via DATABASE_URL is not available, it should report an error.

    // This test is more of a conceptual outline given the `sqlx prepare` constraint.
    // To truly test this, one would typically:
    // 1. Ensure `sqlx prepare` can run (e.g., with a temporary DB).
    // 2. For this specific test, ensure the DB is *not* available at runtime
    //    OR pass a `PgPool` specifically configured with an invalid target.

    // For the purpose of this exercise, we will create a pool with an invalid URL
    // and attempt to configure the app with it. This might fail early.
    // The core issue is that `health_check` uses `sqlx::query` which needs `sqlx-data.json`.

    // let app_config = oxidizedoasis_websands::infrastructure::config::app_config::AppConfig::new();
    // Create a pool that is guaranteed to fail.
    // This is problematic because App::new().app_data() expects an existing pool.
    // If we pass a pool that is known to be bad, the health check should report it.
    // The problem is that `sqlx::query("SELECT 1")` in health.rs needs `sqlx-data.json`.

    // Let's assume for the sake of writing *some* test code:
    // We'll try to initialize the app with a pool that will fail to connect.
    // This will only work if the application's `create_pool` is NOT used,
    // or if we can pass a bad URL to it.

    // The most straightforward way to test the "Error" state of database_status
    // is to ensure the database is *actually* down when the test runs,
    // and that `sqlx prepare` was run against a *valid* database schema previously.
    // Since I cannot control `sqlx prepare` success or DB state:

    // This test will be written with the assumption that if the app starts
    // and the DB (as configured by create_pool()) is down, it should report "Error".
    // This is an integration test of the actual endpoint.
    println!("Running test_health_check_database_error: This test expects the database to be INACCESSIBLE.");
    println!("If a database is running and accessible at the configured DATABASE_URL, this test will FAIL.");

    dotenv::from_filename(".env.test").ok();
    // We don't create a special pool here; we rely on the app's default pool creation.
    // This test becomes an environmental test: is the DB (that the app would use) down?
    // For this test to be meaningful, the DATABASE_URL should point to a non-operational DB.
    // This is not something the test itself can easily enforce without external setup.

    // We will proceed as if the database is unavailable.
    // The real test would involve configuring the app with a pool known to be bad,
    // or ensuring the actual DB is down.
    // Let's just call the endpoint and hope the environment is such that the DB is down.
    // This is not a robust test.

    // A better way: configure a pool with an invalid URL for *this specific test*.
    // let bad_pool = sqlx::PgPool::connect("postgres://nouser:nopass@nohost:1234/nodb").await;
    // `connect` itself will error. We need a `PgPool` instance.
    // `PgPoolOptions::connect_lazy` might be what we need.
    let lazy_pool_options = sqlx::postgres::PgPoolOptions::new().max_connections(1);
    let bad_db_url = "postgres://user:pass@invalid-host-for-test:5432/db";
    let failing_pool = lazy_pool_options.connect_lazy(&bad_db_url).unwrap();


    let mut app = actix_web::test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(failing_pool.clone())) // Use the failing pool
            .configure(oxidizedoasis_websands::api::routes::health::configure) // Only health route
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
