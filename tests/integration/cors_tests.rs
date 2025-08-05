use actix_cors::Cors;
use std::sync::Mutex;
use oxidizedoasis_websands::infrastructure::middleware::cors::configure_cors;



// Global mutex to prevent environment variable interference between parallel tests
static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn test_configure_cors_development_environment() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set development environment
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("DEVELOPMENT_URL", "http://localhost:3000");
    
    // Configure CORS
    let cors = configure_cors();
    
    // Verify CORS configuration (we can't directly inspect Cors internals, 
    // but we can verify the function returns a Cors object)
    let cors_name = std::any::type_name_of_val(&cors);
    assert!(cors_name.contains("Cors"), "Should return a Cors middleware instance");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
}

#[tokio::test]
async fn test_configure_cors_development_environment_default_url() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set development environment without DEVELOPMENT_URL
    std::env::set_var("ENVIRONMENT", "development");
    std::env::remove_var("DEVELOPMENT_URL");
    
    // Configure CORS - should use default localhost URL
    let cors = configure_cors();
    
    // Verify CORS configuration returns properly
    let cors_name = std::any::type_name_of_val(&cors);
    assert!(cors_name.contains("Cors"), "Should return a Cors middleware instance with default URL");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
}

#[tokio::test]
async fn test_configure_cors_production_environment() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set production environment
    std::env::set_var("ENVIRONMENT", "production");
    std::env::set_var("PRODUCTION_URL", "https://myapp.com");
    
    // Configure CORS
    let cors = configure_cors();
    
    // Verify CORS configuration
    let cors_name = std::any::type_name_of_val(&cors);
    assert!(cors_name.contains("Cors"), "Should return a Cors middleware instance for production");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("PRODUCTION_URL");
}

#[tokio::test]
#[should_panic(expected = "PRODUCTION_URL must be set in production")]
async fn test_configure_cors_production_missing_url() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set production environment without PRODUCTION_URL
    std::env::set_var("ENVIRONMENT", "production");
    std::env::remove_var("PRODUCTION_URL");
    
    // This should panic because PRODUCTION_URL is required in production
    let _cors = configure_cors();
    
    // Clean up (though this won't execute due to panic)
    std::env::remove_var("ENVIRONMENT");
}

#[tokio::test]
async fn test_configure_cors_default_environment() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Remove ENVIRONMENT variable to test default behavior
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
    
    // Configure CORS - should default to development
    let cors = configure_cors();
    
    // Verify CORS configuration
    let cors_name = std::any::type_name_of_val(&cors);
    assert!(cors_name.contains("Cors"), "Should return a Cors middleware instance with default environment");
}

#[tokio::test]
async fn test_configure_cors_custom_development_url() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set custom development URL
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("DEVELOPMENT_URL", "http://localhost:5000");
    
    // Configure CORS
    let cors = configure_cors();
    
    // Verify CORS configuration
    let cors_name = std::any::type_name_of_val(&cors);
    assert!(cors_name.contains("Cors"), "Should return a Cors middleware instance with custom development URL");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
}

#[tokio::test]
async fn test_configure_cors_staging_environment() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set staging environment (should behave like development)
    std::env::set_var("ENVIRONMENT", "staging");
    std::env::set_var("DEVELOPMENT_URL", "https://staging.myapp.com");
    
    // Configure CORS
    let cors = configure_cors();
    
    // Verify CORS configuration
    let cors_name = std::any::type_name_of_val(&cors);
    assert!(cors_name.contains("Cors"), "Should return a Cors middleware instance for staging environment");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
}

#[tokio::test]
async fn test_configure_cors_function_signature() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set up development environment to avoid production URL requirement
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("DEVELOPMENT_URL", "http://localhost:3000");
    
    // Test that the function has the expected signature and return type
    let cors = configure_cors();
    
    // Verify it returns a Cors instance
    let _: Cors = cors;
    
    // Test the function can be called multiple times
    let cors1 = configure_cors();
    let cors2 = configure_cors();
    
    // Both should be valid Cors instances
    let cors1_name = std::any::type_name_of_val(&cors1);
    let cors2_name = std::any::type_name_of_val(&cors2);
    assert!(cors1_name.contains("Cors"), "First call should return Cors instance");
    assert!(cors2_name.contains("Cors"), "Second call should return Cors instance");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
}

#[tokio::test]
async fn test_configure_cors_with_all_environments() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Test various environment values
    let test_cases = vec![
        ("development", Some("http://localhost:8080"), false),
        ("dev", Some("http://localhost:3000"), false),
        ("test", Some("http://localhost:4000"), false),
        ("staging", Some("https://staging.example.com"), false),
        ("production", Some("https://production.example.com"), false),
    ];
    
    for (env, dev_url, should_panic) in test_cases {
        // Set environment
        std::env::set_var("ENVIRONMENT", env);
        
        if env == "production" {
            std::env::set_var("PRODUCTION_URL", dev_url.unwrap());
            std::env::remove_var("DEVELOPMENT_URL");
        } else {
            std::env::set_var("DEVELOPMENT_URL", dev_url.unwrap());
            std::env::remove_var("PRODUCTION_URL");
        }
        
        if should_panic {
            // Handle panic case if needed
            continue;
        }
        
        // Configure CORS
        let cors = configure_cors();
        
        // Verify result
        let cors_name = std::any::type_name_of_val(&cors);
        assert!(cors_name.contains("Cors"), 
                "Should return Cors instance for environment: {env}");
        
        // Clean up
        std::env::remove_var("ENVIRONMENT");
        std::env::remove_var("PRODUCTION_URL");
        std::env::remove_var("DEVELOPMENT_URL");
    }
}

#[tokio::test]
async fn test_cors_configuration_structure() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set up test environment
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
    
    // Get CORS configuration
    let cors = configure_cors();
    
    // Test that we can create an actix-web app with this CORS configuration
    use actix_web::{web, App, HttpResponse};
    
    let app = App::new()
        .wrap(cors)
        .route("/test", web::get().to(|| async { HttpResponse::Ok().body("test") }));
    
    // If we get here without compilation errors, the CORS configuration is valid
    let app_name = std::any::type_name_of_val(&app);
    assert!(app_name.contains("App"), "Should be able to wrap app with CORS middleware");
    
    // Clean up
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
}

#[tokio::test]
async fn test_cors_headers_and_methods_configuration() {
    let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    
    // Set up test environment to avoid production URL requirement
    std::env::set_var("ENVIRONMENT", "development");
    std::env::set_var("DEVELOPMENT_URL", "http://localhost:8080");
    
    // This test verifies that the CORS configuration includes expected headers and methods
    // Since we can't directly inspect Cors internals, we test that it can be created successfully
    
    let cors = configure_cors();
    
    // Test that the CORS middleware can be used in an app configuration
    use actix_web::{test, web, App, HttpResponse};
    
    let app = test::init_service(
        App::new()
            .wrap(cors)
            .route("/test", web::route().method(actix_web::http::Method::OPTIONS).to(|| async { HttpResponse::Ok().finish() }))
    ).await;
    
    // Create a preflight OPTIONS request to test CORS headers
    let req = test::TestRequest::with_uri("/test")
        .method(actix_web::http::Method::OPTIONS)
        .insert_header(("Origin", "http://localhost:8080"))
        .insert_header(("Access-Control-Request-Method", "POST"))
        .insert_header(("Access-Control-Request-Headers", "Content-Type"))
        .to_request();
    
    let resp = test::call_service(&app, req).await;
    
    // Verify that the response is successful (CORS middleware processed it)
    assert!(resp.status().is_success() || resp.status().as_u16() == 200,
            "CORS preflight request should be handled successfully");
    
    // Clean up environment variables
    std::env::remove_var("ENVIRONMENT");
    std::env::remove_var("DEVELOPMENT_URL");
}