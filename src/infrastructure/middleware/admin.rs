use actix_web::error::ResponseError;
use actix_web::{dev::ServiceRequest, Error, HttpMessage, HttpResponse, web}; // Added web for app_data
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde_json::json;
use std::fmt;
use std::sync::Arc; // For Arc
use log::{error, debug, warn};
use crate::core::auth::jwt::{validate_jwt, TokenType};
use crate::core::auth::token_revocation::TokenRevocationServiceTrait; // Import the trait

#[derive(Debug)]
pub enum AdminError {
    Unauthorized(String),
    Forbidden(String),
}

impl fmt::Display for AdminError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AdminError::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            AdminError::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
        }
    }
}

impl ResponseError for AdminError {
    fn error_response(&self) -> HttpResponse {
        match self {
            AdminError::Unauthorized(message) => {
                HttpResponse::Unauthorized().json(json!({
                    "error": "Unauthorized",
                    "message": message
                }))
            },
            AdminError::Forbidden(message) => {
                HttpResponse::Forbidden().json(json!({
                    "error": "Forbidden",
                    "message": message
                }))
            }
        }
    }
}

pub async fn admin_validator(req: ServiceRequest, credentials: BearerAuth) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    debug!("Admin validator called for path: {}", req.path());
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let token = credentials.token();

    // Extract TokenRevocationService from app_data
    let token_revocation_service = req.app_data::<web::Data<Arc<dyn TokenRevocationServiceTrait>>>().cloned();
    if token_revocation_service.is_none() {
        error!("TokenRevocationService not found in app_data for admin_validator");
        return Err((AdminError::Forbidden(
            "Internal server configuration error".to_string()
        ).into(), req));
    }
    let token_revocation_service = token_revocation_service.unwrap().into_inner(); // Get Arc<dyn Trait>

    debug!("Attempting to validate token for admin access");

    // Get expected audience and issuer for token validation
    let expected_audience = std::env::var("JWT_AUDIENCE").ok();
    let expected_issuer = std::env::var("JWT_ISSUER").ok();
    
    debug!("Admin middleware - expected_audience: {expected_audience:?}, expected_issuer: {expected_issuer:?}");
    eprintln!("🔍 ADMIN MIDDLEWARE DEBUG - expected_audience: {expected_audience:?}, expected_issuer: {expected_issuer:?}");
    
    // Validate as an access token - we don't accept refresh tokens for API access
    let validation_result = validate_jwt(&token_revocation_service, token, &jwt_secret, Some(TokenType::Access), expected_audience, expected_issuer).await;
    
    match validation_result {
        Ok(claims) => {
            // Check if the user has admin role
            if claims.role != "admin" {
                error!("Access denied: User {} with role {} attempted to access admin endpoint",
                       claims.sub, claims.role);
                return Err((AdminError::Forbidden(
                    "Access denied: Insufficient privileges".to_string()
                ).into(), req));
            }
            
            // Check token expiration time and warn if it's close to expiring
            let now = chrono::Utc::now().timestamp();
            let remaining_time = claims.exp - now;
            
            if remaining_time < 300 { // Less than 5 minutes remaining
                warn!("Admin token for user {} is about to expire in {} seconds", claims.sub, remaining_time);
            }
            
            debug!("Admin access granted for user: {}", claims.sub);
            // Add claims to request extensions for use in handlers
            req.extensions_mut().insert(claims);
            Ok(req)
        },
        Err(e) => {
            error!("Token validation failed: {e:?}");
            Err((AdminError::Unauthorized(
                "Invalid or expired token".to_string()
            ).into(), req))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web::Data, App, Error as ActixError, HttpMessage, FromRequest}; // Added FromRequest
    use actix_web::http::{header::{self, HeaderValue, AUTHORIZATION}, StatusCode}; // Import AUTHORIZATION
    use actix_web_httpauth::headers::authorization::{Authorization, Bearer}; // For constructing BearerAuth
    use serde_json::Value;
    use std::sync::Arc;
    use crate::core::auth::jwt::{Claims, TokenType as JwtTokenType}; // Claims and TokenType
    // create_jwt is not directly used by test token generation, using jsonwebtoken::encode instead for simplicity
    use crate::core::auth::token_revocation::TokenRevocationServiceTrait; 
    use mockall::mock;
    use sqlx::Error as SqlxError; 
    use async_trait::async_trait;
    use uuid::Uuid;
    use std::env;
    use std::sync::Mutex;
    use chrono::{Utc, Duration};

    // Mutex to ensure serial execution of tests modifying environment variables
    static ENV_MUTEX: Mutex<()> = Mutex::new(());
    
    const TEST_JWT_SECRET: &str = "test_secret_key_for_admin_middleware";

    // Helper to run an async test with multiple env vars set
    async fn run_test_with_env_vars<F, Fut>(vars: Vec<(&str, Option<&str>)>, test_fn: F) -> Fut::Output
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future,
    {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|poisoned| {
            // Clear the poison and continue - this allows tests to recover from panics
            let guard = poisoned.into_inner();
            guard
        });
        let mut original_values = Vec::new();

        // Store original values
        for (key, _) in &vars {
            original_values.push((key.to_string(), env::var(key).ok()));
        }

        // Set new values
        for (key, value) in vars {
            if let Some(v) = value {
                env::set_var(key, v);
            } else {
                env::remove_var(key);
            }
        }

        // Run the async test function
        let result = test_fn().await;

        // Restore original values
        for (key, original_value) in original_values {
            if let Some(orig_val) = original_value {
                env::set_var(key, orig_val);
            } else {
                env::remove_var(&key);
            }
        }
        result
    }

    
    mock! {
        pub TokenRevocationService {}
        #[async_trait]
        impl TokenRevocationServiceTrait for TokenRevocationService {
            async fn revoke_token<'a>(&self, jti: &'a str, user_id: Uuid, token_type: JwtTokenType, expires_at: chrono::DateTime<Utc>, reason: Option<&'a str>) -> Result<(), SqlxError>;
            async fn is_token_revoked(&self, jti: &str) -> Result<bool, SqlxError>;
            async fn cleanup_expired_tokens(&self) -> Result<u64, SqlxError>;
            async fn revoke_all_user_tokens<'a>(&self, user_id: Uuid, reason: Option<&'a str>) -> Result<u64, SqlxError>;
        }
    }

    fn create_test_claims_for_middleware(user_id: Uuid, role: &str, exp_duration_secs: i64, aud: &str, iss: &str) -> Claims {
        let now = Utc::now();
        let iat_ts = now.timestamp();
        Claims { // Added aud and iss fields
            aud: aud.to_string(),
            iss: iss.to_string(),
            sub: user_id,
            role: role.to_string(),
            exp: (now + Duration::seconds(exp_duration_secs)).timestamp(),
            iat: iat_ts,
            nbf: iat_ts, // nbf usually same as iat or slightly before
            jti: Uuid::new_v4().to_string(),
            token_type: JwtTokenType::Access, // Middleware expects Access token
        }
    }

    // generate_test_token using the helper for claims
    fn generate_test_token_middleware(user_id: Uuid, role: &str, secret: &str, exp_duration_secs: i64, aud: &str, iss: &str) -> String {
        let claims = create_test_claims_for_middleware(user_id, role, exp_duration_secs, aud, iss);
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_ref()),
        ).expect("Failed to generate test token for middleware")
    }

    #[actix_rt::test] // Make this test async
    async fn test_admin_error_response() {
        let admin_error_forbidden = AdminError::Forbidden("Test forbidden message".to_string());
        let http_response_forbidden = admin_error_forbidden.error_response();
        assert_eq!(http_response_forbidden.status(), StatusCode::FORBIDDEN);

        let srv_res_forbidden = test::TestRequest::default().to_srv_response(http_response_forbidden);
        let body_forbidden = test::read_body_json::<Value, _>(srv_res_forbidden).await;

        assert_eq!(body_forbidden["error"], "Forbidden");
        assert_eq!(body_forbidden["message"], "Test forbidden message");

        let admin_error_unauthorized = AdminError::Unauthorized("Test unauthorized message".to_string());
        let http_response_unauthorized = admin_error_unauthorized.error_response();
        assert_eq!(http_response_unauthorized.status(), StatusCode::UNAUTHORIZED);

        let srv_res_unauthorized = test::TestRequest::default().to_srv_response(http_response_unauthorized);
        let body_unauthorized = test::read_body_json::<Value, _>(srv_res_unauthorized).await;

        assert_eq!(body_unauthorized["error"], "Unauthorized");
        assert_eq!(body_unauthorized["message"], "Test unauthorized message");
    }
    
    #[actix_rt::test]
    async fn test_admin_validator_valid_admin_token() {
        let mut mock_revocation_service = MockTokenRevocationService::new();
        mock_revocation_service.expect_is_token_revoked()
            .returning(|_| Ok(false)); 

        let app_data_revocation_service = Data::new(Arc::new(mock_revocation_service) as Arc<dyn TokenRevocationServiceTrait>);
        
        let user_id = Uuid::new_v4();
        let test_aud = "test_aud";
        let test_iss = "test_iss";
        let token_str = generate_test_token_middleware(user_id, "admin", TEST_JWT_SECRET, 3600, test_aud, test_iss);

        // Create a ServiceRequest for extracting BearerAuth
        let srv_req_for_extraction = test::TestRequest::default()
            .insert_header((AUTHORIZATION, format!("Bearer {}", token_str)))
            .to_srv_request();
        let (http_req, mut payload) = srv_req_for_extraction.into_parts();
        let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();

        // Create a new ServiceRequest for the validator function, configured with necessary app_data
        let srv_req_for_validator = test::TestRequest::default()
            .app_data(app_data_revocation_service.clone())
            // No need to set auth header here as admin_validator receives BearerAuth directly
            .to_srv_request();
        
        let env_vars = vec![
            ("JWT_SECRET", Some(TEST_JWT_SECRET)),
            ("JWT_AUDIENCE", Some(test_aud)),
            ("JWT_ISSUER", Some(test_iss))
        ];
        run_test_with_env_vars(env_vars, || async {
            let result = admin_validator(srv_req_for_validator, bearer_auth).await;
            assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
            let srv_req = result.unwrap();
            // To ensure extensions() lives long enough for get()
            let claims = {
                let extensions_map = srv_req.extensions();
                extensions_map.get::<Claims>().cloned() // Clone if Claims is Clone, or handle reference carefully
            }.unwrap(); // Assuming Claims is Clone. If not, this needs adjustment.
            // If Claims is not Clone, then assert directly:
            // assert!(srv_req.extensions().get::<Claims>().is_some());
            // let claims_ref = srv_req.extensions().get::<Claims>().unwrap();
            // assert_eq!(claims_ref.sub, user_id);
            // assert_eq!(claims_ref.role, "admin");
            // Given Claims derives Clone, cloning is safer.
            assert_eq!(claims.sub, user_id);
            assert_eq!(claims.role, "admin");
        }).await;
    }

    #[actix_rt::test]
    async fn test_admin_validator_non_admin_token() {
        let mut mock_revocation_service = MockTokenRevocationService::new();
        mock_revocation_service.expect_is_token_revoked()
            .returning(|_| Ok(false));
        let app_data_revocation_service = Data::new(Arc::new(mock_revocation_service) as Arc<dyn TokenRevocationServiceTrait>);

        let user_id = Uuid::new_v4();
        let test_aud = "test_aud";
        let test_iss = "test_iss";
        let token_str = generate_test_token_middleware(user_id, "user", TEST_JWT_SECRET, 3600, test_aud, test_iss);

        let srv_req_for_extraction = test::TestRequest::default()
            .insert_header((AUTHORIZATION, format!("Bearer {}", token_str)))
            .to_srv_request();
        let (http_req, mut payload) = srv_req_for_extraction.into_parts();
        let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();

        let srv_req_for_validator = test::TestRequest::default()
            .app_data(app_data_revocation_service.clone())
            .to_srv_request();

        let env_vars = vec![
            ("JWT_SECRET", Some(TEST_JWT_SECRET)),
            ("JWT_AUDIENCE", Some(test_aud)),
            ("JWT_ISSUER", Some(test_iss))
        ];
        run_test_with_env_vars(env_vars, || async {
            let result = admin_validator(srv_req_for_validator, bearer_auth).await;
            assert!(result.is_err(), "Expected Err for non-admin token");
            let (err, _) = result.err().unwrap(); 
            let http_response = err.error_response(); 
            assert_eq!(http_response.status(), StatusCode::FORBIDDEN);
            let srv_res = test::TestRequest::default().to_srv_response(http_response); // Convert to ServiceResponse
            let body = test::read_body_json::<serde_json::Value, _>(srv_res).await;
            assert_eq!(body["error"], "Forbidden");
            assert_eq!(body["message"], "Access denied: Insufficient privileges");
        }).await;
    }

    #[actix_rt::test]
    async fn test_admin_validator_token_revoked() {
        let mut mock_revocation_service = MockTokenRevocationService::new();
        mock_revocation_service.expect_is_token_revoked()
            .returning(|_| Ok(true)); 
        let app_data_revocation_service = Data::new(Arc::new(mock_revocation_service) as Arc<dyn TokenRevocationServiceTrait>);

        let user_id = Uuid::new_v4();
        let test_aud = "test_aud";
        let test_iss = "test_iss";
        let token_str = generate_test_token_middleware(user_id, "admin", TEST_JWT_SECRET, 3600, test_aud, test_iss);

        let srv_req_for_extraction = test::TestRequest::default()
            .insert_header((AUTHORIZATION, format!("Bearer {}", token_str)))
            .to_srv_request();
        let (http_req, mut payload) = srv_req_for_extraction.into_parts();
        let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
        
        let srv_req_for_validator = test::TestRequest::default()
            .app_data(app_data_revocation_service.clone())
            .to_srv_request();
        
        let env_vars = vec![("JWT_SECRET", Some(TEST_JWT_SECRET))];
        run_test_with_env_vars(env_vars, || async {
            let result = admin_validator(srv_req_for_validator, bearer_auth).await;
            assert!(result.is_err(), "Expected Err for revoked token");
            let (err, _) = result.err().unwrap();
            let http_response = err.error_response();
            assert_eq!(http_response.status(), StatusCode::UNAUTHORIZED);
            let srv_res = test::TestRequest::default().to_srv_response(http_response); // Convert to ServiceResponse
            let body = test::read_body_json::<serde_json::Value, _>(srv_res).await;
            assert_eq!(body["error"], "Unauthorized");
            assert_eq!(body["message"], "Invalid or expired token");
        }).await;
    }
    
    #[actix_rt::test]
    async fn test_admin_validator_revocation_service_missing_in_app_data() {
        let user_id = Uuid::new_v4();
        let test_aud = "test_aud";
        let test_iss = "test_iss";
        let token_str = generate_test_token_middleware(user_id, "admin", TEST_JWT_SECRET, 3600, test_aud, test_iss);

        let srv_req_for_extraction = test::TestRequest::default()
            .insert_header((AUTHORIZATION, format!("Bearer {}", token_str)))
            .to_srv_request();
        let (http_req, mut payload) = srv_req_for_extraction.into_parts();
        let bearer_auth = BearerAuth::from_request(&http_req, &mut payload).await.unwrap();
        
        // ServiceRequest for validator, without the TokenRevocationService in app_data
        let srv_req_for_validator = test::TestRequest::default().to_srv_request(); 
        
        let env_vars = vec![("JWT_SECRET", Some(TEST_JWT_SECRET))];
        run_test_with_env_vars(env_vars, || async {
            let result = admin_validator(srv_req_for_validator, bearer_auth).await;
            assert!(result.is_err(), "Expected Err when revocation service is missing");
            let (err, _) = result.err().unwrap();
            let http_response = err.error_response();
            assert_eq!(http_response.status(), StatusCode::FORBIDDEN);
            let srv_res = test::TestRequest::default().to_srv_response(http_response); // Convert to ServiceResponse
            let body = test::read_body_json::<serde_json::Value, _>(srv_res).await;
            assert_eq!(body["error"], "Forbidden");
            assert_eq!(body["message"], "Internal server configuration error");
        }).await;
    }

    // Note: Testing JWT_SECRET missing would cause a panic due to .expect()
    // A more robust implementation might return an error, which could then be tested.
}
