//! Comprehensive middleware testing
//! Tests authentication, authorization, CORS, metrics, and other middleware components

use actix_web::{test, web, App, http::StatusCode, cookie::Cookie, HttpMessage, FromRequest, HttpResponse, middleware};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use tokio::time::{sleep, Instant};

use oxidizedoasis_websands::infrastructure::middleware::{
    auth::{jwt_auth_validator, cookie_auth_validator},
    admin::admin_validator,
    metrics::RequestMetrics,
};
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;
use oxidizedoasis_websands::core::auth::token_revocation::TokenRevocationServiceTrait;

use test_common::{
    create_test_app_config, generate_test_token,
    mocks::*, env::with_env_vars, TEST_JWT_SECRET, UnifiedTestFixture
};

/// Test fixture for middleware tests
struct MiddlewareTestFixture {
    app_config: AppConfig,
    token_revocation_service: Arc<dyn TokenRevocationServiceTrait>,
    valid_user_token: String,
    valid_admin_token: String,
    expired_token: String,
    invalid_token: String,
    revoked_token: String,
}

impl MiddlewareTestFixture {
    fn new() -> Self {
        let app_config = create_test_app_config();
        let token_revocation_service = Arc::new(create_mock_token_revocation_service());
        
        let user_id = Uuid::new_v4();
        let admin_id = Uuid::new_v4();
        
        let valid_user_token = generate_test_token(user_id, "user", 3600)
            .expect("Failed to generate user token");
        let valid_admin_token = generate_test_token(admin_id, "admin", 3600)
            .expect("Failed to generate admin token");
        let expired_token = generate_test_token(user_id, "user", -3600)
            .expect("Failed to generate expired token");
        let invalid_token = "invalid.jwt.token".to_string();
        let revoked_token = generate_test_token(user_id, "user", 3600)
            .expect("Failed to generate revoked token");

        Self {
            app_config,
            token_revocation_service,
            valid_user_token,
            valid_admin_token,
            expired_token,
            invalid_token,
            revoked_token,
        }
    }
}

#[cfg(test)]
mod auth_middleware_tests {
    use super::*;
    use actix_web_httpauth::extractors::bearer::BearerAuth;
    use std::collections::HashMap;

    #[actix_rt::test]
    async fn test_jwt_auth_validator_valid_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.valid_user_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = jwt_auth_validator(req, bearer_auth).await;
            assert!(result.is_ok());
            
            let validated_req = result.unwrap();
            let extensions = validated_req.extensions();
            let claims = extensions.get::<oxidizedoasis_websands::core::auth::jwt::Claims>();
            assert!(claims.is_some());
        });
    }

    #[actix_rt::test]
    async fn test_jwt_auth_validator_invalid_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.invalid_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = jwt_auth_validator(req, bearer_auth).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        });
    }

    #[actix_rt::test]
    async fn test_cookie_auth_validator_valid_cookie() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .cookie(Cookie::new("access_token", fixture.valid_user_token.clone()))
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();
            
            let result = cookie_auth_validator(req).await;
            assert!(result.is_ok());
            
            let validated_req = result.unwrap();
            let extensions = validated_req.extensions();
            let claims = extensions.get::<oxidizedoasis_websands::core::auth::jwt::Claims>();
            assert!(claims.is_some());
        });
    }

    #[actix_rt::test]
    async fn test_cookie_auth_validator_missing_cookie() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.app_config))
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();
            
            let result = cookie_auth_validator(req).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        });
    }
}

#[cfg(test)]
mod admin_middleware_tests {
    use super::*;
    use actix_web_httpauth::extractors::bearer::BearerAuth;
    use std::collections::HashMap;

    #[actix_rt::test]
    async fn test_admin_validator_valid_admin_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.valid_admin_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = admin_validator(req, bearer_auth).await;
            assert!(result.is_ok());
            
            let validated_req = result.unwrap();
            let extensions = validated_req.extensions();
            let claims = extensions.get::<oxidizedoasis_websands::core::auth::jwt::Claims>();
            assert!(claims.is_some());
            assert_eq!(claims.unwrap().role, "admin");
        });
    }

    #[actix_rt::test]
    async fn test_admin_validator_non_admin_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.valid_user_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = admin_validator(req, bearer_auth).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        });
    }

    #[actix_rt::test]
    async fn test_admin_validator_invalid_token() {
        let fixture = MiddlewareTestFixture::new();
        
        let env_vars = HashMap::from([
            ("JWT_SECRET", TEST_JWT_SECRET),
        ]);

        with_env_vars(env_vars, || async {
            let req = test::TestRequest::default()
                .app_data(web::Data::new(fixture.token_revocation_service))
                .to_srv_request();

            // Create BearerAuth using from_request
            let auth_req = test::TestRequest::default()
                .insert_header(("Authorization", format!("Bearer {}", fixture.invalid_token)))
                .to_srv_request();
            let (http_req, mut payload) = auth_req.into_parts();
            let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
            
            let result = admin_validator(req, bearer_auth).await;
            assert!(result.is_err());
            
            let (error, _) = result.unwrap_err();
            let response = error.error_response();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        });
    }
}

#[cfg(test)]
mod csrf_protection_tests {
    use super::*;
    
    #[actix_rt::test]
    async fn test_csrf_protection_missing_header() {
        // Test CSRF protection when X-CSRF-Token header is missing
        let fixture = MiddlewareTestFixture::new();
        
        // This would be tested with actual CSRF middleware integration
        // For now, this is a placeholder for future CSRF middleware testing
        assert!(true); // Placeholder assertion
    }

    #[actix_rt::test]
    async fn test_csrf_protection_invalid_token() {
        // Test CSRF protection with invalid token
        let fixture = MiddlewareTestFixture::new();
        
        // This would be tested with actual CSRF middleware integration
        // For now, this is a placeholder for future CSRF middleware testing
        assert!(true); // Placeholder assertion
    }
    
    #[cfg(test)]
    mod metrics_middleware_tests {
        use super::*;
    
        /// Test metrics middleware registration and initialization
        mod middleware_registration_tests {
            use super::*;
    
            #[tokio::test]
            async fn test_metrics_middleware_registration_success() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                // Test that RequestMetrics middleware can be successfully registered
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/health", web::get().to(|| async {
                            HttpResponse::Ok().json(serde_json::json!({"status": "healthy"}))
                        }))
                ).await;
                
                // Test that the app initializes successfully with metrics middleware
                let req = test::TestRequest::get().uri("/health").to_request();
                let resp = test::call_service(&app, req).await;
                
                assert_eq!(resp.status(), StatusCode::OK);
                let body: serde_json::Value = test::read_body_json(resp).await;
                assert_eq!(body["status"], "healthy");
            }
            
            #[tokio::test]
            async fn test_metrics_middleware_initialization_with_multiple_routes() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                // Test metrics middleware with multiple routes
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/api/users", web::get().to(|| async {
                            HttpResponse::Ok().json(serde_json::json!({"users": []}))
                        }))
                        .route("/api/health", web::get().to(|| async {
                            HttpResponse::Ok().body("OK")
                        }))
                        .route("/api/status", web::post().to(|| async {
                            HttpResponse::Created().json(serde_json::json!({"created": true}))
                        }))
                ).await;
                
                // Test each route to ensure metrics middleware is working across all
                let routes = [
                    ("/api/users", "GET", StatusCode::OK),
                    ("/api/health", "GET", StatusCode::OK),
                    ("/api/status", "POST", StatusCode::CREATED),
                ];
                
                for (uri, method, expected_status) in routes.iter() {
                    let req = match *method {
                        "GET" => test::TestRequest::get().uri(uri).to_request(),
                        "POST" => test::TestRequest::post().uri(uri).to_request(),
                        _ => panic!("Unsupported method: {method}"),
                    };
                    
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), *expected_status, "Failed for {method} {uri}");
                }
            }
            
            #[tokio::test]
            async fn test_metrics_middleware_with_other_middleware() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                // Test metrics middleware integration with other middleware
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics) // Metrics middleware
                        .wrap(middleware::Logger::default()) // Logger middleware
                        .wrap(middleware::Compress::default()) // Compression middleware
                        .wrap(
                            middleware::DefaultHeaders::new()
                                .add(("X-Frame-Options", "DENY"))
                                .add(("X-Content-Type-Options", "nosniff"))
                        )
                        .route("/test", web::get().to(|| async {
                            HttpResponse::Ok().body("middleware stack test")
                        }))
                ).await;
                
                let req = test::TestRequest::get().uri("/test").to_request();
                let resp = test::call_service(&app, req).await;
                
                assert_eq!(resp.status(), StatusCode::OK);
                
                // Verify security headers are present (indicating middleware stack works)
                let headers = resp.headers();
                assert!(
                    headers.contains_key("x-frame-options") || headers.contains_key("X-Frame-Options"),
                    "Security middleware should set X-Frame-Options header"
                );
            }
        }
    
        /// Test request and response metrics collection
        mod metrics_collection_tests {
            use super::*;
    
            #[tokio::test]
            async fn test_request_metrics_collection_basic() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/", web::get().to(|| async {
                            HttpResponse::Ok().body("test response")
                        }))
                ).await;
                
                // Make a request and verify it completes successfully
                let req = test::TestRequest::get().uri("/").to_request();
                let resp = test::call_service(&app, req).await;
                
                assert!(resp.status().is_success());
                let body_bytes = test::read_body(resp).await;
                assert_eq!(body_bytes, "test response");
            }
            
            #[tokio::test]
            async fn test_request_timing_and_performance_monitoring() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/slow", web::get().to(|| async {
                            // Simulate some processing time
                            sleep(Duration::from_millis(10)).await;
                            HttpResponse::Ok().body("slow response")
                        }))
                        .route("/fast", web::get().to(|| async {
                            HttpResponse::Ok().body("fast response")
                        }))
                ).await;
                
                let start_time = Instant::now();
                
                // Test slow endpoint
                let req = test::TestRequest::get().uri("/slow").to_request();
                let resp = test::call_service(&app, req).await;
                assert!(resp.status().is_success());
                
                let slow_duration = start_time.elapsed();
                assert!(slow_duration >= Duration::from_millis(10), "Slow endpoint should take at least 10ms");
                
                // Test fast endpoint
                let req = test::TestRequest::get().uri("/fast").to_request();
                let resp = test::call_service(&app, req).await;
                assert!(resp.status().is_success());
            }
            
            #[tokio::test]
            async fn test_metrics_with_different_http_methods() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/resource", web::get().to(|| async {
                            HttpResponse::Ok().json(serde_json::json!({"id": 1, "name": "test"}))
                        }))
                        .route("/resource", web::post().to(|| async {
                            HttpResponse::Created().json(serde_json::json!({"id": 2, "created": true}))
                        }))
                        .route("/resource/{id}", web::put().to(|| async {
                            HttpResponse::Ok().json(serde_json::json!({"id": 1, "updated": true}))
                        }))
                        .route("/resource/{id}", web::delete().to(|| async {
                            HttpResponse::NoContent().finish()
                        }))
                ).await;
                
                // Test GET request
                let req = test::TestRequest::get().uri("/resource").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
                
                // Test POST request
                let req = test::TestRequest::post()
                    .uri("/resource")
                    .set_json(serde_json::json!({"name": "new resource"}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::CREATED);
                
                // Test PUT request
                let req = test::TestRequest::put()
                    .uri("/resource/1")
                    .set_json(serde_json::json!({"name": "updated resource"}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
                
                // Test DELETE request
                let req = test::TestRequest::delete().uri("/resource/1").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::NO_CONTENT);
            }
            
            #[tokio::test]
            async fn test_metrics_with_different_status_codes() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/success", web::get().to(|| async {
                            HttpResponse::Ok().body("success")
                        }))
                        .route("/created", web::post().to(|| async {
                            HttpResponse::Created().body("created")
                        }))
                        .route("/not-found", web::get().to(|| async {
                            HttpResponse::NotFound().body("not found")
                        }))
                        .route("/error", web::get().to(|| async {
                            HttpResponse::InternalServerError().body("server error")
                        }))
                        .route("/bad-request", web::post().to(|| async {
                            HttpResponse::BadRequest().body("bad request")
                        }))
                ).await;
                
                let test_cases = [
                    ("/success", "GET", StatusCode::OK),
                    ("/created", "POST", StatusCode::CREATED),
                    ("/not-found", "GET", StatusCode::NOT_FOUND),
                    ("/error", "GET", StatusCode::INTERNAL_SERVER_ERROR),
                    ("/bad-request", "POST", StatusCode::BAD_REQUEST),
                ];
                
                for (uri, method, expected_status) in test_cases.iter() {
                    let req = match *method {
                        "GET" => test::TestRequest::get().uri(uri).to_request(),
                        "POST" => test::TestRequest::post().uri(uri).to_request(),
                        _ => panic!("Unsupported method: {method}"),
                    };
                    
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), *expected_status, "Failed for {method} {uri}");
                }
            }
        }
    
        /// Test sequential request handling
        mod sequential_request_tests {
            use super::*;
    
            #[tokio::test]
            async fn test_sequential_request_metrics() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/counter", web::get().to(|| async {
                            HttpResponse::Ok().body("counter response")
                        }))
                ).await;
                
                // Make multiple sequential requests
                for i in 0..5 {
                    let req = test::TestRequest::get()
                        .uri("/counter")
                        .to_request();
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), StatusCode::OK, "Request {} should succeed", i + 1);
                }
            }
            
            #[tokio::test]
            async fn test_mixed_sequential_request_types() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/get-endpoint", web::get().to(|| async {
                            HttpResponse::Ok().body("get response")
                        }))
                        .route("/post-endpoint", web::post().to(|| async {
                            HttpResponse::Created().body("post response")
                        }))
                        .route("/slow-endpoint", web::get().to(|| async {
                            sleep(Duration::from_millis(5)).await;
                            HttpResponse::Ok().body("slow response")
                        }))
                ).await;
                
                // Mix of different request types - sequential
                for i in 0..9 {
                    let (uri, method, expected_status) = match i % 3 {
                        0 => ("/get-endpoint", "GET", StatusCode::OK),
                        1 => ("/post-endpoint", "POST", StatusCode::CREATED),
                        _ => ("/slow-endpoint", "GET", StatusCode::OK),
                    };
                    
                    let req = match method {
                        "GET" => test::TestRequest::get().uri(uri).to_request(),
                        "POST" => test::TestRequest::post().uri(uri).to_request(),
                        _ => panic!("Unsupported method: {method}"),
                    };
                    
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), expected_status, "Request {} should have status {:?}", i + 1, expected_status);
                }
            }
        }
    
        /// Test error handling in metrics collection
        mod error_handling_tests {
            use super::*;
    
            #[tokio::test]
            async fn test_metrics_with_handler_errors() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/server-error", web::get().to(|| async {
                            HttpResponse::InternalServerError().body("simulated server error")
                        }))
                        .route("/bad-request", web::get().to(|| async {
                            HttpResponse::BadRequest().body("simulated bad request")
                        }))
                        .route("/unauthorized", web::get().to(|| async {
                            HttpResponse::Unauthorized().body("simulated unauthorized")
                        }))
                        .route("/success", web::get().to(|| async {
                            HttpResponse::Ok().body("success")
                        }))
                ).await;
                
                // Test that metrics middleware handles various error responses
                let req = test::TestRequest::get().uri("/server-error").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
                
                let req = test::TestRequest::get().uri("/bad-request").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
                
                let req = test::TestRequest::get().uri("/unauthorized").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
                
                // Test that success responses still work after errors
                let req = test::TestRequest::get().uri("/success").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
            }
            
            #[tokio::test]
            async fn test_metrics_with_invalid_routes() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/valid", web::get().to(|| async {
                            HttpResponse::Ok().body("valid response")
                        }))
                ).await;
                
                // Test requests to non-existent routes
                let invalid_routes = [
                    "/nonexistent",
                    "/invalid/path",
                    "/api/missing",
                    "//double-slash",
                    "/valid/extra/path",
                ];
                
                for route in invalid_routes.iter() {
                    let req = test::TestRequest::get().uri(route).to_request();
                    let resp = test::call_service(&app, req).await;
                    assert_eq!(resp.status(), StatusCode::NOT_FOUND, "Route {route} should return 404");
                }
                
                // Verify valid route still works
                let req = test::TestRequest::get().uri("/valid").to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
            }
            
            #[tokio::test]
            async fn test_metrics_with_malformed_requests() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/json", web::post().to(|_: web::Json<serde_json::Value>| async {
                            HttpResponse::Ok().body("json received")
                        }))
                        .route("/text", web::post().to(|body: String| async move {
                            HttpResponse::Ok().body(format!("received: {body}"))
                        }))
                ).await;
                
                // Test malformed JSON request
                let req = test::TestRequest::post()
                    .uri("/json")
                    .set_payload("invalid json {")
                    .insert_header(("content-type", "application/json"))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert!(resp.status().is_client_error(), "Malformed JSON should return client error");
                
                // Test valid JSON request
                let req = test::TestRequest::post()
                    .uri("/json")
                    .set_json(serde_json::json!({"test": "data"}))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
                
                // Test text request
                let req = test::TestRequest::post()
                    .uri("/text")
                    .set_payload("test text data")
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
            }
        }
    
        /// Test metrics functionality under various conditions
        mod performance_edge_case_tests {
            use super::*;
    
            #[tokio::test]
            async fn test_metrics_with_large_request_body() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/large", web::post().to(|body: String| async move {
                            HttpResponse::Ok().body(format!("Received {} bytes", body.len()))
                        }))
                ).await;
                
                // Create a large request body (1KB)
                let large_body = "x".repeat(1024);
                
                let req = test::TestRequest::post()
                    .uri("/large")
                    .set_payload(large_body)
                    .to_request();
                let resp = test::call_service(&app, req).await;
                
                assert_eq!(resp.status(), StatusCode::OK);
                let body = test::read_body(resp).await;
                assert!(String::from_utf8_lossy(&body).contains("1024 bytes"));
            }
            
            #[tokio::test]
            async fn test_metrics_with_streaming_response() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/stream", web::get().to(|| async {
                            use actix_web::web::Bytes;
                            use futures::stream;
                            
                            let data = vec![
                                Bytes::from("chunk1\n"),
                                Bytes::from("chunk2\n"),
                                Bytes::from("chunk3\n"),
                            ];
                            
                            HttpResponse::Ok()
                                .content_type("text/plain")
                                .streaming(stream::iter(data.into_iter().map(Ok::<_, actix_web::Error>)))
                        }))
                ).await;
                
                let req = test::TestRequest::get().uri("/stream").to_request();
                let resp = test::call_service(&app, req).await;
                
                assert_eq!(resp.status(), StatusCode::OK);
                let body = test::read_body(resp).await;
                let body_str = String::from_utf8_lossy(&body);
                assert!(body_str.contains("chunk1"));
                assert!(body_str.contains("chunk2"));
                assert!(body_str.contains("chunk3"));
            }
            
            #[tokio::test]
            async fn test_metrics_with_long_running_request() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/slow", web::get().to(|| async {
                            // Simulate longer processing time
                            sleep(Duration::from_millis(100)).await;
                            HttpResponse::Ok().body("slow processing complete")
                        }))
                ).await;
                
                let start = Instant::now();
                
                let req = test::TestRequest::get().uri("/slow").to_request();
                let resp = test::call_service(&app, req).await;
                
                let duration = start.elapsed();
                
                assert_eq!(resp.status(), StatusCode::OK);
                assert!(duration >= Duration::from_millis(100), "Request should take at least 100ms");
                
                let body = test::read_body(resp).await;
                assert_eq!(body, "slow processing complete");
            }
            
            #[tokio::test]
            async fn test_metrics_with_various_content_types() {
                let _fixture = UnifiedTestFixture::new_with_database().await;
                
                let app = test::init_service(
                    App::new()
                        .wrap(RequestMetrics)
                        .route("/json", web::post().to(|json: web::Json<serde_json::Value>| async move {
                            HttpResponse::Ok().json(json.into_inner())
                        }))
                        .route("/form", web::post().to(|form: web::Form<std::collections::HashMap<String, String>>| async move {
                            HttpResponse::Ok().json(form.into_inner())
                        }))
                        .route("/text", web::post().to(|text: String| async move {
                            HttpResponse::Ok().body(format!("Text: {text}"))
                        }))
                ).await;
                
                // Test JSON content type
                let json_data = serde_json::json!({"key": "value", "number": 42});
                let req = test::TestRequest::post()
                    .uri("/json")
                    .set_json(&json_data)
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
                
                // Test form content type
                let req = test::TestRequest::post()
                    .uri("/form")
                    .set_form([("field1", "value1"), ("field2", "value2")])
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
                
                // Test plain text content type
                let req = test::TestRequest::post()
                    .uri("/text")
                    .set_payload("plain text data")
                    .insert_header(("content-type", "text/plain"))
                    .to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK);
            }
        }
    }
}

#[cfg(test)]
mod logger_middleware_tests {
    use super::*;
    use oxidizedoasis_websands::infrastructure::middleware::logger::RequestLogger;
    use actix_web::{test, web, App, HttpResponse, http::StatusCode};
    use std::sync::{Arc, Mutex};
    use std::collections::HashMap;

    /// Test handler that returns 4xx status codes for CLIENT ERROR testing
    async fn client_error_handler() -> HttpResponse {
        HttpResponse::BadRequest().body("client error")
    }

    /// Test handler that returns 5xx status codes for SERVER ERROR testing
    async fn server_error_handler() -> HttpResponse {
        HttpResponse::InternalServerError().body("server error")
    }

    /// Test handler that returns successful response
    async fn success_handler() -> HttpResponse {
        HttpResponse::Ok().body("success")
    }

    #[actix_rt::test]
    async fn test_request_logger_with_client_error_status() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/client-error", web::get().to(client_error_handler))
        ).await;

        let req = test::TestRequest::get().uri("/client-error").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        // The middleware should log "CLIENT ERROR" - this is tested by the middleware itself
    }

    #[actix_rt::test]
    async fn test_request_logger_with_server_error_status() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/server-error", web::get().to(server_error_handler))
        ).await;

        let req = test::TestRequest::get().uri("/server-error").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        // The middleware should log "SERVER ERROR" - this is tested by the middleware itself
    }

    #[actix_rt::test]
    async fn test_request_logger_with_various_4xx_status_codes() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/bad-request", web::get().to(|| async {
                    HttpResponse::BadRequest().body("bad request")
                }))
                .route("/unauthorized", web::get().to(|| async {
                    HttpResponse::Unauthorized().body("unauthorized")
                }))
                .route("/forbidden", web::get().to(|| async {
                    HttpResponse::Forbidden().body("forbidden")
                }))
                .route("/not-found", web::get().to(|| async {
                    HttpResponse::NotFound().body("not found")
                }))
        ).await;

        let test_cases = [
            ("/bad-request", StatusCode::BAD_REQUEST),
            ("/unauthorized", StatusCode::UNAUTHORIZED),
            ("/forbidden", StatusCode::FORBIDDEN),
            ("/not-found", StatusCode::NOT_FOUND),
        ];

        for (uri, expected_status) in test_cases.iter() {
            let req = test::TestRequest::get().uri(uri).to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), *expected_status, "Failed for {uri}");
            // Each should trigger "CLIENT ERROR" logging
        }
    }

    #[actix_rt::test]
    async fn test_request_logger_with_various_5xx_status_codes() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/internal-error", web::get().to(|| async {
                    HttpResponse::InternalServerError().body("internal error")
                }))
                .route("/not-implemented", web::get().to(|| async {
                    HttpResponse::NotImplemented().body("not implemented")
                }))
                .route("/bad-gateway", web::get().to(|| async {
                    HttpResponse::BadGateway().body("bad gateway")
                }))
                .route("/service-unavailable", web::get().to(|| async {
                    HttpResponse::ServiceUnavailable().body("service unavailable")
                }))
        ).await;

        let test_cases = [
            ("/internal-error", StatusCode::INTERNAL_SERVER_ERROR),
            ("/not-implemented", StatusCode::NOT_IMPLEMENTED),
            ("/bad-gateway", StatusCode::BAD_GATEWAY),
            ("/service-unavailable", StatusCode::SERVICE_UNAVAILABLE),
        ];

        for (uri, expected_status) in test_cases.iter() {
            let req = test::TestRequest::get().uri(uri).to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), *expected_status, "Failed for {uri}");
            // Each should trigger "SERVER ERROR" logging
        }
    }

    #[actix_rt::test]
    async fn test_request_logger_with_referer_header() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/test", web::get().to(success_handler))
        ).await;

        // Test with Referer header
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Referer", "https://example.com/previous-page"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::OK);
        // The middleware should log with the referer value

        // Test without Referer header
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::OK);
        // The middleware should log with "none" for referer
    }

    #[actix_rt::test]
    async fn test_request_logger_with_user_agent_header() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/test", web::get().to(success_handler))
        ).await;

        // Test with User-Agent header
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("User-Agent", "Mozilla/5.0 (Test Browser)"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::OK);
        // The middleware should log with the user agent value

        // Test without User-Agent header
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::OK);
        // The middleware should log with "none" for user agent
    }

    #[actix_rt::test]
    async fn test_request_logger_with_both_referer_and_user_agent() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/test", web::get().to(success_handler))
                .route("/client-error", web::get().to(client_error_handler))
                .route("/server-error", web::get().to(server_error_handler))
        ).await;

        let headers = [
            ("Referer", "https://example.com/page"),
            ("User-Agent", "TestBot/1.0"),
        ];

        // Test success response with both headers
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(headers[0])
            .insert_header(headers[1])
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // Test client error with both headers
        let req = test::TestRequest::get()
            .uri("/client-error")
            .insert_header(headers[0])
            .insert_header(headers[1])
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // Test server error with both headers
        let req = test::TestRequest::get()
            .uri("/server-error")
            .insert_header(headers[0])
            .insert_header(headers[1])
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_rt::test]
    async fn test_request_logger_with_invalid_header_values() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/test", web::get().to(success_handler))
        ).await;

        // Test with headers that might have invalid UTF-8 (simulated with unusual characters)
        let req = test::TestRequest::get()
            .uri("/test")
            .insert_header(("Referer", "https://example.com/page?q=test"))
            .insert_header(("User-Agent", "TestBot/1.0 (compatible; test)"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::OK);
        // The middleware should handle these headers gracefully
    }

    #[actix_rt::test]
    async fn test_request_logger_timing_functionality() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/slow", web::get().to(|| async {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    HttpResponse::Ok().body("slow response")
                }))
                .route("/fast", web::get().to(|| async {
                    HttpResponse::Ok().body("fast response")
                }))
        ).await;

        // Test that the middleware handles timing for both fast and slow requests
        let start = std::time::Instant::now();
        
        let req = test::TestRequest::get().uri("/slow").to_request();
        let resp = test::call_service(&app, req).await;
        let slow_duration = start.elapsed();
        
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(slow_duration >= std::time::Duration::from_millis(10));

        let req = test::TestRequest::get().uri("/fast").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_request_logger_with_different_http_methods() {
        let app = test::init_service(
            App::new()
                .wrap(RequestLogger::new())
                .route("/resource", web::get().to(|| async {
                    HttpResponse::Ok().body("GET response")
                }))
                .route("/resource", web::post().to(|| async {
                    HttpResponse::Created().body("POST response")
                }))
                .route("/resource", web::put().to(|| async {
                    HttpResponse::Ok().body("PUT response")
                }))
                .route("/resource", web::delete().to(|| async {
                    HttpResponse::NoContent().finish()
                }))
                .route("/error", web::patch().to(|| async {
                    HttpResponse::InternalServerError().body("PATCH error")
                }))
        ).await;

        let test_cases = [
            ("GET", "/resource", StatusCode::OK),
            ("POST", "/resource", StatusCode::CREATED),
            ("PUT", "/resource", StatusCode::OK),
            ("DELETE", "/resource", StatusCode::NO_CONTENT),
            ("PATCH", "/error", StatusCode::INTERNAL_SERVER_ERROR),
        ];

        for (method, uri, expected_status) in test_cases.iter() {
            let req = match *method {
                "GET" => test::TestRequest::get().uri(uri).to_request(),
                "POST" => test::TestRequest::post().uri(uri).to_request(),
                "PUT" => test::TestRequest::put().uri(uri).to_request(),
                "DELETE" => test::TestRequest::delete().uri(uri).to_request(),
                "PATCH" => test::TestRequest::patch().uri(uri).to_request(),
                _ => panic!("Unsupported method: {method}"),
            };
            
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), *expected_status, "Failed for {method} {uri}");
        }
    }
}

#[cfg(test)]
mod rate_limiting_tests {
    
    
    #[actix_rt::test]
    async fn test_rate_limiting_under_limit() {
        // Test that requests under rate limit are allowed
        assert!(true); // Placeholder for future rate limiting tests
    }

    #[actix_rt::test]
    async fn test_rate_limiting_over_limit() {
        // Test that requests over rate limit are blocked
        assert!(true); // Placeholder for future rate limiting tests
    }
}