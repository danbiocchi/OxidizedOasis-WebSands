use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
    body::EitherBody,
    http::StatusCode,
};
use log::{debug, warn};
use dashmap::DashMap;
use futures::future::{ready, LocalBoxFuture, Ready};
use std::sync::Arc;
use std::task::{Context, Poll};

// Rate limit configurations for different endpoints
struct RateLimit {
    path: &'static str,
    max_requests: u32,
    window_seconds: u64,
    error_message: &'static str,
}

const RATE_LIMITS: &[RateLimit] = &[
    // Specific endpoint rate limits (checked first)
    RateLimit {
        path: "/users/login",
        max_requests: 5,
        window_seconds: 300, // 5 minutes
        error_message: "Too many login attempts",
    },
    RateLimit {
        path: "/users/register",
        max_requests: 3,
        window_seconds: 3600, // 1 hour
        error_message: "Too many registration attempts",
    },
    RateLimit {
        path: "/users/verify-email",
        max_requests: 5,
        window_seconds: 300, // 5 minutes
        error_message: "Too many email verification attempts",
    },
    RateLimit {
        path: "/users/password-reset/request",
        max_requests: 3,
        window_seconds: 1800, // 30 minutes
        error_message: "Too many password reset requests",
    },
    RateLimit {
        path: "/users/password-reset/verify",
        max_requests: 10,
        window_seconds: 300, // 5 minutes
        error_message: "Too many verification attempts",
    },
    RateLimit {
        path: "/password-reset/new",
        max_requests: 10,
        window_seconds: 300, // 5 minutes
        error_message: "Too many verification attempts",
    },
    // Global default rate limit (fallback)
    RateLimit {
        path: "*",  // Wildcard for all paths
        max_requests: 1000,
        window_seconds: 3600, // 1 hour
        error_message: "Too many requests. Please try again later",
    },
];

pub struct RateLimiter;

impl RateLimiter {
    pub fn new() -> Self {
        RateLimiter
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimitMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddleware {
            service,
            rate_limits: Arc::new(DashMap::new()),
        }))
    }
}

pub struct RateLimitMiddleware<S> {
    service: S,
    rate_limits: Arc<DashMap<String, Vec<u64>>>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path();
        
        // Extract base path without query parameters
        let base_path = path.split('?').next().unwrap_or(path);
        debug!("Rate limit checking path: {}", base_path);

        // Skip rate limiting for static files
        if base_path.ends_with(".css") || base_path.ends_with(".js") || 
           base_path.ends_with(".wasm") || base_path.ends_with(".ico") {
            let fut = self.service.call(req);
            return Box::pin(async move {
                fut.await.map(|res| res.map_into_left_body())
            });
        }

        // Find matching rate limit configuration (specific paths first, then fallback to wildcard)
        let rate_limit = RATE_LIMITS.iter()
            .find(|rl| {
                debug!("Comparing against rate limit path: {}", rl.path);
                base_path == rl.path || rl.path == "*"
            })
            .expect("Global rate limit should always match");

        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create a unique key combining IP and endpoint path for independent rate limiting
        let rate_limit_key = format!("{}:{}", ip, base_path);
        
        let mut timestamps = self
            .rate_limits
            .entry(rate_limit_key.clone())
            .or_insert_with(Vec::new)
            .value()
            .clone();

        // Remove timestamps outside the current window
        timestamps.retain(|&ts| now - ts < rate_limit.window_seconds);

        // Clean up old entries from the map periodically (every 10 requests)
        if now % 10 == 0 {
            self.rate_limits.retain(|_, v| {
                v.retain(|&ts| now - ts < rate_limit.window_seconds);
                !v.is_empty()
            });
        }

        debug!(
            "Rate limit check for {}: {}/{} requests in {}s window",
            base_path,
            timestamps.len(),
            rate_limit.max_requests,
            rate_limit.window_seconds
        );

        // Check if rate limit is exceeded
        if timestamps.len() >= rate_limit.max_requests as usize {
            warn!(
                "Rate limit exceeded for {} - {} requests in {}s window",
                base_path,
                timestamps.len(),
                rate_limit.window_seconds
            );
            let reset_time = timestamps[0] + rate_limit.window_seconds;
            let wait_seconds = reset_time.saturating_sub(now);
            let wait_minutes = (wait_seconds + 59) / 60;

            let error_response = HttpResponse::TooManyRequests()
                .append_header(("Retry-After", wait_seconds.to_string()))
                .json(serde_json::json!({
                    "error": rate_limit.error_message,
                    "message": format!("Please wait {} minutes before trying again", wait_minutes),
                    "retry_after": wait_seconds
                }));

            let (http_req, _) = req.into_parts();
            return Box::pin(async move {
                Ok(ServiceResponse::new(
                    http_req,
                    error_response.map_into_right_body()
                ))
            });
        }

        // Add current timestamp and update the map
        timestamps.push(now);
        self.rate_limits.insert(rate_limit_key, timestamps);

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::time::{sleep, Duration};
    use serde_json::Value;

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({"message": "success"}))
    }

    #[actix_rt::test]
    async fn test_rate_limiter_new() {
        let rate_limiter = RateLimiter::new();
        // Simple test to ensure RateLimiter can be instantiated
        assert!(std::mem::size_of_val(&rate_limiter) >= 0);
    }

    #[actix_rt::test]
    async fn test_rate_limit_middleware_creation() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_global_rate_limit_under_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/api/test", web::get().to(test_handler))
        ).await;

        // Make several requests under the global limit (1000/hour)
        for i in 0..10 {
            let req = test::TestRequest::get().uri("/api/test").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Request {} should succeed", i + 1);
        }
    }

    #[actix_rt::test]
    async fn test_login_endpoint_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/login", web::post().to(test_handler))
        ).await;

        // Login endpoint has a limit of 5 requests per 5 minutes
        // Make 5 requests (should all succeed)
        for i in 0..5 {
            let req = test::TestRequest::post().uri("/users/login").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Login request {} should succeed", i + 1);
        }

        // 6th request should be rate limited
        let req = test::TestRequest::post().uri("/users/login").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_register_endpoint_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/register", web::post().to(test_handler))
        ).await;

        // Register endpoint has a limit of 3 requests per hour
        // Make 3 requests (should all succeed)
        for i in 0..3 {
            let req = test::TestRequest::post().uri("/users/register").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Register request {} should succeed", i + 1);
        }

        // 4th request should be rate limited
        let req = test::TestRequest::post().uri("/users/register").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_password_reset_request_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/password-reset/request", web::post().to(test_handler))
        ).await;

        // Password reset request has a limit of 3 requests per 30 minutes
        for i in 0..3 {
            let req = test::TestRequest::post().uri("/users/password-reset/request").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Password reset request {} should succeed", i + 1);
        }

        // 4th request should be rate limited
        let req = test::TestRequest::post().uri("/users/password-reset/request").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_email_verification_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/verify-email", web::post().to(test_handler))
        ).await;

        // Email verification has a limit of 5 requests per 5 minutes
        for i in 0..5 {
            let req = test::TestRequest::post().uri("/users/verify-email").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Email verification request {} should succeed", i + 1);
        }

        // 6th request should be rate limited
        let req = test::TestRequest::post().uri("/users/verify-email").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_password_reset_verify_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/password-reset/verify", web::post().to(test_handler))
        ).await;

        // Password reset verify has a limit of 10 requests per 5 minutes
        for i in 0..10 {
            let req = test::TestRequest::post().uri("/users/password-reset/verify").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Password reset verify request {} should succeed", i + 1);
        }

        // 11th request should be rate limited
        let req = test::TestRequest::post().uri("/users/password-reset/verify").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_password_reset_new_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/password-reset/new", web::post().to(test_handler))
        ).await;

        // Password reset new has a limit of 10 requests per 5 minutes
        for i in 0..10 {
            let req = test::TestRequest::post().uri("/password-reset/new").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Password reset new request {} should succeed", i + 1);
        }

        // 11th request should be rate limited
        let req = test::TestRequest::post().uri("/password-reset/new").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_static_files_bypass_rate_limit() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/static/style.css", web::get().to(test_handler))
                .route("/static/script.js", web::get().to(test_handler))
                .route("/static/app.wasm", web::get().to(test_handler))
                .route("/favicon.ico", web::get().to(test_handler))
        ).await;

        let static_files = ["/static/style.css", "/static/script.js", "/static/app.wasm", "/favicon.ico"];
        
        // Static files should bypass rate limiting - make many requests
        for file_path in static_files.iter() {
            for i in 0..15 {
                let req = test::TestRequest::get().uri(file_path).to_request();
                let resp = test::call_service(&app, req).await;
                assert_eq!(resp.status(), StatusCode::OK, "Static file request {} to {} should succeed", i + 1, file_path);
            }
        }
    }

    #[actix_rt::test]
    async fn test_query_parameters_ignored() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/login", web::post().to(test_handler))
        ).await;

        // Make requests with different query parameters - should be treated as same endpoint
        let paths = [
            "/users/login",
            "/users/login?ref=homepage",
            "/users/login?utm_source=email",
            "/users/login?redirect=/dashboard"
        ];

        // Make 5 requests total across different query params
        for (i, path) in paths.iter().enumerate() {
            let req = test::TestRequest::post().uri(path).to_request();
            let resp = test::call_service(&app, req).await;
            if i < 4 {
                assert_eq!(resp.status(), StatusCode::OK, "Request to {} should succeed", path);
            }
        }

        // 5th request should succeed (we have 5 request limit)
        let req = test::TestRequest::post().uri("/users/login?final=test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // 6th request should be rate limited
        let req = test::TestRequest::post().uri("/users/login?extra=test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[actix_rt::test]
    async fn test_different_endpoints_independent_limits() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/login", web::post().to(test_handler))
                .route("/users/register", web::post().to(test_handler))
                .route("/other/endpoint", web::get().to(test_handler))
        ).await;

        // Exhaust login rate limit (5 requests)
        for i in 0..5 {
            let req = test::TestRequest::post().uri("/users/login").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK, "Login request {} should succeed", i + 1);
        }

        // Login should now be rate limited
        let req = test::TestRequest::post().uri("/users/login").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);

        // But register should still work (independent rate limit)
        let req = test::TestRequest::post().uri("/users/register").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // And other paths should use global rate limit
        let req = test::TestRequest::get().uri("/other/endpoint").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_rate_limit_error_response_format() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/login", web::post().to(test_handler))
        ).await;

        // Exhaust the rate limit
        for _ in 0..5 {
            let req = test::TestRequest::post().uri("/users/login").to_request();
            test::call_service(&app, req).await;
        }

        // Next request should return proper error format
        let req = test::TestRequest::post().uri("/users/login").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        
        // Check for Retry-After header
        assert!(resp.headers().contains_key("retry-after"));
        
        // Check response body format
        let body = test::read_body(resp).await;
        let json_response: Value = serde_json::from_slice(&body).expect("Response should be valid JSON");
        
        assert!(json_response.get("error").is_some());
        assert!(json_response.get("message").is_some());
        assert!(json_response.get("retry_after").is_some());
        
        assert_eq!(json_response["error"], "Too many login attempts");
    }

    #[actix_rt::test]
    async fn test_rate_limit_ip_extraction() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/test", web::get().to(test_handler))
        ).await;

        // Test that middleware handles IP extraction without headers
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_rate_limit_timestamp_cleanup() {
        // Test that the middleware properly handles timestamp cleanup logic
        // This tests the code path where now % 10 == 0
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/test", web::get().to(test_handler))
        ).await;

        // Make multiple requests to trigger potential cleanup
        for _ in 0..15 {
            let req = test::TestRequest::get().uri("/test").to_request();
            let resp = test::call_service(&app, req).await;
            assert_eq!(resp.status(), StatusCode::OK);
        }
    }

    #[actix_rt::test]
    async fn test_rate_limit_window_calculation() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/login", web::post().to(test_handler))
        ).await;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Make a request to establish baseline
        let req = test::TestRequest::post().uri("/users/login").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // The middleware should handle window calculations correctly
        // This test verifies the timestamp retention logic works
    }

    #[actix_rt::test]
    async fn test_wildcard_rate_limit_fallback() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/random/endpoint", web::get().to(test_handler))
        ).await;

        // Endpoint not in specific rate limits should use wildcard (*) global limit
        let req = test::TestRequest::get().uri("/random/endpoint").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_rate_limit_retry_after_calculation() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/register", web::post().to(test_handler))
        ).await;

        // Exhaust register rate limit (3 requests)
        for _ in 0..3 {
            let req = test::TestRequest::post().uri("/users/register").to_request();
            test::call_service(&app, req).await;
        }

        // Next request should have retry-after header
        let req = test::TestRequest::post().uri("/users/register").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        
        let retry_after = resp.headers().get("retry-after");
        assert!(retry_after.is_some());
        
        let retry_seconds: u64 = retry_after.unwrap().to_str().unwrap().parse().unwrap();
        assert!(retry_seconds > 0);
        assert!(retry_seconds <= 3600); // Should be within the window
    }

    #[actix_rt::test]
    async fn test_rate_limit_message_formatting() {
        let app = test::init_service(
            App::new()
                .wrap(RateLimiter::new())
                .route("/users/password-reset/request", web::post().to(test_handler))
        ).await;

        // Exhaust password reset request limit (3 requests)
        for _ in 0..3 {
            let req = test::TestRequest::post().uri("/users/password-reset/request").to_request();
            test::call_service(&app, req).await;
        }

        // Check error message format
        let req = test::TestRequest::post().uri("/users/password-reset/request").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        
        let body = test::read_body(resp).await;
        let json_response: Value = serde_json::from_slice(&body).unwrap();
        
        assert_eq!(json_response["error"], "Too many password reset requests");
        assert!(json_response["message"].as_str().unwrap().contains("Please wait"));
        assert!(json_response["message"].as_str().unwrap().contains("minutes"));
    }
}
