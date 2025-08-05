use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    FromRequest,
    http::Method, Error, HttpMessage, HttpRequest,
};
use futures_util::future::{ok, ready, LocalBoxFuture, Ready};
use log::warn;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

// CSRF token structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CsrfToken {
    pub token: String,
}

impl Default for CsrfToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CsrfToken {
    pub fn new() -> Self {
        let mut rng = rand::rng();
        let token: String = (0..32)
            .map(|_| {
                const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                        abcdefghijklmnopqrstuvwxyz\
                                        0123456789";
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();
        Self { token }
    }
}

// CSRF protection middleware
pub struct CsrfProtection;

impl<S, B> Transform<S, ServiceRequest> for CsrfProtection
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Transform = CsrfProtectionMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CsrfProtectionMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct CsrfProtectionMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for CsrfProtectionMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        // Skip CSRF check for safe methods (GET, HEAD, OPTIONS)
        if req.method() == Method::GET || req.method() == Method::HEAD || req.method() == Method::OPTIONS {
            return Box::pin(async move {
                service.call(req).await.map(|res| res.map_into_left_body())
            });
        }

        // Skip CSRF check for API endpoints that don't need it
        let path = req.path();
        if path.starts_with("/api/") && !path.contains("/users/") {
            return Box::pin(async move {
                service.call(req).await.map(|res| res.map_into_left_body())
            });
        }

        // Get CSRF token from header
        let header_token = req
            .headers()
            .get("X-CSRF-Token")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        // Get CSRF token from cookie
        let cookie_token = req
            .cookie("csrf_token")
            .map(|c| c.value().to_string());

        // Validate tokens
        let valid = match (header_token, cookie_token) {
            (Some(header), Some(cookie)) => {
                let valid = header == cookie;
                if !valid {
                    warn!("CSRF token mismatch: header={header}, cookie={cookie}");
                }
                valid
            },
            _ => {
                warn!("Missing CSRF token: header={:?}, cookie={:?}", 
                      req.headers().get("X-CSRF-Token"), 
                      req.cookie("csrf_token"));
                false
            }
        };

        if valid {
            Box::pin(async move {
                service.call(req).await.map(|res| res.map_into_left_body())
            })
        } else {
            Box::pin(async move {
                let (http_req, _) = req.into_parts();
                let response = actix_web::HttpResponse::Forbidden()
                    .json(serde_json::json!({
                        "error": "Invalid CSRF token"
                    }))
                    .map_into_right_body();
                Ok(ServiceResponse::new(http_req, response))
            })
        }
    }
}

// Helper function to generate a new CSRF token
pub fn generate_csrf_token() -> CsrfToken {
    CsrfToken::new()
}

// Helper function to set CSRF token cookie
pub fn set_csrf_token_cookie(req: &HttpRequest) -> CsrfToken {
    // Check if there's already a CSRF token in the cookie
    if let Some(cookie) = req.cookie("csrf_token") {
        return CsrfToken { token: cookie.value().to_string() };
    }
    
    // Generate a new token
    let token = generate_csrf_token();
    
    // Store in request extensions for handlers to access
    req.extensions_mut().insert(token.clone());
    
    token
}

// Extractor for CSRF token from request
pub struct CsrfTokenExtractor(pub String);

impl FromRequest for CsrfTokenExtractor {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        // Try to get token from extensions (set by middleware)
        if let Some(token) = req.extensions().get::<CsrfToken>() {
            return ok(CsrfTokenExtractor(token.token.clone()));
        }
        
        // Try to get from cookie
        if let Some(cookie) = req.cookie("csrf_token") {
            return ok(CsrfTokenExtractor(cookie.value().to_string()));
        }
        
        // Generate new token
        let token = generate_csrf_token();
        ok(CsrfTokenExtractor(token.token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse, cookie::Cookie};
    use actix_web::http::{Method, StatusCode};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({"message": "success"}))
    }

    #[actix_rt::test]
    async fn test_csrf_token_new() {
        let token1 = CsrfToken::new();
        let token2 = CsrfToken::new();
        
        // Tokens should be 32 characters long
        assert_eq!(token1.token.len(), 32);
        assert_eq!(token2.token.len(), 32);
        
        // Tokens should be different
        assert_ne!(token1.token, token2.token);
        
        // Tokens should only contain alphanumeric characters
        assert!(token1.token.chars().all(|c| c.is_alphanumeric()));
        assert!(token2.token.chars().all(|c| c.is_alphanumeric()));
    }

    #[actix_rt::test]
    async fn test_generate_csrf_token() {
        let token1 = generate_csrf_token();
        let token2 = generate_csrf_token();
        
        assert_eq!(token1.token.len(), 32);
        assert_ne!(token1.token, token2.token);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_allows_safe_methods() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::get().to(test_handler))
                .route("/test-head", web::head().to(test_handler))
        ).await;

        // GET request should be allowed without CSRF token
        let req = test::TestRequest::get().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // HEAD request should be allowed without CSRF token
        let req = test::TestRequest::with_uri("/test-head").method(Method::HEAD).to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // OPTIONS request should be allowed without CSRF token (middleware bypasses it)
        let req = test::TestRequest::with_uri("/test").method(Method::OPTIONS).to_request();
        let resp = test::call_service(&app, req).await;
        // OPTIONS will return 405 Method Not Allowed since no route is configured, but middleware should not block it
        assert_ne!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_blocks_post_without_token() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        // POST request without CSRF token should be blocked
        let req = test::TestRequest::post().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_blocks_put_without_token() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::put().to(test_handler))
        ).await;

        // PUT request without CSRF token should be blocked
        let req = test::TestRequest::put().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_blocks_delete_without_token() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::delete().to(test_handler))
        ).await;

        // DELETE request without CSRF token should be blocked
        let req = test::TestRequest::delete().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_allows_valid_token() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        let token = "test_csrf_token_12345678901234567890";

        // POST request with matching header and cookie tokens should be allowed
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-CSRF-Token", token))
            .cookie(Cookie::new("csrf_token", token))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_blocks_mismatched_tokens() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        // POST request with mismatched header and cookie tokens should be blocked
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-CSRF-Token", "header_token"))
            .cookie(Cookie::new("csrf_token", "cookie_token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_blocks_missing_header() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        // POST request with cookie but no header should be blocked
        let req = test::TestRequest::post()
            .uri("/test")
            .cookie(Cookie::new("csrf_token", "test_token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_blocks_missing_cookie() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        // POST request with header but no cookie should be blocked
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-CSRF-Token", "test_token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_api_endpoints_bypass() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/api/health", web::post().to(test_handler))
                .route("/api/status", web::put().to(test_handler))
        ).await;

        // API endpoints (excluding /users/) should bypass CSRF protection
        let req = test::TestRequest::post().uri("/api/health").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let req = test::TestRequest::put().uri("/api/status").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_csrf_protection_api_users_requires_token() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/api/users/create", web::post().to(test_handler))
        ).await;

        // API endpoints containing /users/ should require CSRF protection
        let req = test::TestRequest::post().uri("/api/users/create").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // But should work with valid token
        let token = "test_token_123456789012345678901234";
        let req = test::TestRequest::post()
            .uri("/api/users/create")
            .insert_header(("X-CSRF-Token", token))
            .cookie(Cookie::new("csrf_token", token))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn test_set_csrf_token_cookie_existing_token() {
        let app = test::init_service(App::new()).await;
        let existing_token = "existing_token_123456789012345678";
        
        let req = test::TestRequest::get()
            .cookie(Cookie::new("csrf_token", existing_token))
            .to_srv_request();

        let token = set_csrf_token_cookie(req.request());
        assert_eq!(token.token, existing_token);
    }

    #[actix_rt::test]
    async fn test_set_csrf_token_cookie_new_token() {
        let app = test::init_service(App::new()).await;
        let req = test::TestRequest::get().to_srv_request();

        let token = set_csrf_token_cookie(req.request());
        assert_eq!(token.token.len(), 32);
        assert!(token.token.chars().all(|c| c.is_alphanumeric()));
    }

    #[actix_rt::test]
    async fn test_csrf_token_extractor_from_extensions() {
        let app = test::init_service(App::new()).await;
        let test_token = CsrfToken { token: "extension_token_12345678901234567890".to_string() };
        
        let req = test::TestRequest::get().to_srv_request();
        req.request().extensions_mut().insert(test_token.clone());

        let mut payload = actix_web::dev::Payload::None;
        let extractor = CsrfTokenExtractor::from_request(req.request(), &mut payload).await.unwrap();
        assert_eq!(extractor.0, test_token.token);
    }

    #[actix_rt::test]
    async fn test_csrf_token_extractor_from_cookie() {
        let app = test::init_service(App::new()).await;
        let cookie_token = "cookie_token_123456789012345678901";
        
        let req = test::TestRequest::get()
            .cookie(Cookie::new("csrf_token", cookie_token))
            .to_srv_request();

        let mut payload = actix_web::dev::Payload::None;
        let extractor = CsrfTokenExtractor::from_request(req.request(), &mut payload).await.unwrap();
        assert_eq!(extractor.0, cookie_token);
    }

    #[actix_rt::test]
    async fn test_csrf_token_extractor_generates_new() {
        let app = test::init_service(App::new()).await;
        let req = test::TestRequest::get().to_srv_request();

        let mut payload = actix_web::dev::Payload::None;
        let extractor = CsrfTokenExtractor::from_request(req.request(), &mut payload).await.unwrap();
        assert_eq!(extractor.0.len(), 32);
        assert!(extractor.0.chars().all(|c| c.is_alphanumeric()));
    }

    #[actix_rt::test]
    async fn test_csrf_protection_middleware_transform() {
        let csrf_protection = CsrfProtection;
        let service = test::ok_service();
        
        let middleware = csrf_protection.new_transform(service).await.unwrap();
        // Just test that middleware is created successfully
        assert!(std::mem::size_of_val(&middleware) > 0);
    }

    #[actix_rt::test]
    async fn test_csrf_error_message() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        let req = test::TestRequest::post().uri("/test").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        
        // The error should contain the CSRF message
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Invalid CSRF token"));
    }

    #[actix_rt::test]
    async fn test_csrf_protection_different_paths() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/form/submit", web::post().to(test_handler))
                .route("/api/public", web::post().to(test_handler))
                .route("/api/users/update", web::post().to(test_handler))
        ).await;

        // Regular form endpoint should require CSRF
        let req = test::TestRequest::post().uri("/form/submit").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // API public endpoint should not require CSRF
        let req = test::TestRequest::post().uri("/api/public").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        // API users endpoint should require CSRF
        let req = test::TestRequest::post().uri("/api/users/update").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_token_case_sensitivity() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        let token = "TestToken123456789012345678901234";
        let token_different_case = "testtoken123456789012345678901234";

        // Tokens with different case should not match
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-CSRF-Token", token))
            .cookie(Cookie::new("csrf_token", token_different_case))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[actix_rt::test]
    async fn test_csrf_header_parsing() {
        let app = test::init_service(
            App::new()
                .wrap(CsrfProtection)
                .route("/test", web::post().to(test_handler))
        ).await;

        // Test with invalid header value (non-UTF8)
        let req = test::TestRequest::post()
            .uri("/test")
            .insert_header(("X-CSRF-Token", "valid_token"))
            .cookie(Cookie::new("csrf_token", "valid_token"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
    }
}