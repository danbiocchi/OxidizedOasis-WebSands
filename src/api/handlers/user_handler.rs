use actix_web::{web, HttpResponse, Responder, http::header, cookie::{Cookie, SameSite}, HttpRequest, HttpMessage};
use sqlx::PgPool;
use uuid::Uuid;
use std::sync::Arc;
use log::{debug, error, info, warn};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use serde_json::json;
use crate::core::auth::jwt::Claims;
use crate::core::user::{UserRepository, UserService};
use crate::core::auth::AuthService;
use crate::core::email::EmailServiceTrait;
use crate::common::error::ApiErrorType;
use crate::core::auth::token_revocation::TokenRevocationServiceTrait; // Added
use crate::core::auth::active_token::ActiveTokenServiceTrait; // Added
use crate::common::validation::{UserInput, LoginInput, RegisterInput, TokenQuery};
use crate::core::user::model::{PasswordResetRequest, PasswordResetSubmit};
use time;
use crate::infrastructure::middleware::csrf::generate_csrf_token;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
}

pub struct UserHandler {
    user_service: Arc<UserService>,
    auth_service: Arc<AuthService>,
}

impl UserHandler {
    pub fn new(
        pool: PgPool,
        email_service: Arc<dyn EmailServiceTrait>,
        auth_service: Arc<AuthService>, // Changed: Receive AuthService
        token_revocation_service: Arc<dyn TokenRevocationServiceTrait>, // Added
        _active_token_service: Arc<dyn ActiveTokenServiceTrait> // Renamed from _active_token_service as it's used by AuthService
    ) -> Self {
        let user_repo: Arc<dyn crate::core::user::UserRepositoryTrait> = Arc::new(UserRepository::new(pool.clone())); // Explicitly type as Arc<dyn Trait>
        // UserService now needs TokenRevocationService
        let user_service = Arc::new(UserService::new(user_repo, email_service, token_revocation_service.clone())); // Pass the correct service
        
        Self {
            user_service,
            auth_service, // Use received AuthService
        }
    }

    pub async fn create_user(
        &self,
        register_input: web::Json<RegisterInput>,
    ) -> impl Responder {
        debug!("Received create_user request");

        // Validate and sanitize input first
        match crate::common::validation::validate_and_sanitize_register_input(register_input.into_inner()) {
            Ok(validated_input) => {
                // Convert RegisterInput to UserInput for the service layer
                let user_input = UserInput {
                    username: validated_input.username,
                    email: Some(validated_input.email),
                    password: Some(validated_input.password),
                };
                match self.user_service.create_user(user_input).await {
                    Ok((user, _)) => {
                        info!("User created successfully: {}", user.id);
                        HttpResponse::Created().json(json!({
                            "success": true,
                            "message": "User created successfully. Please check your email for verification.",
                            "data": {
                                "user": {
                                    "id": user.id,
                                    "username": user.username,
                                    "email": user.email,
                                    "is_email_verified": user.is_email_verified,
                                    "created_at": user.created_at,
                                    "is_active": user.is_active
                                }
                            }
                        }))
                    },
                    Err(e) => {
                        error!("Failed to create user: {:?}", e);
                        HttpResponse::BadRequest().json(json!({
                            "success": false,
                            "message": e.to_string(),
                            "error": e.to_string()
                        }))
                    }
                }
            },
            Err(validation_errors) => {
                error!("Validation failed for create_user: {:?}", validation_errors);
                let combined_message = format!("Validation error: {:?}", validation_errors);
                
                HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": combined_message,
                    "error": "Validation failed"
                }))
            }
        }
    }

    pub async fn login_user(
        &self,
        login_input: web::Json<LoginInput>,
    ) -> impl Responder {
        match self.auth_service.login(login_input.into_inner()).await {
            Ok((token_pair, user)) => {
                info!("User logged in successfully: {}", user.id);
                
                // Create response
                let mut response = HttpResponse::Ok();
                
                // Set cookies
                let domain = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let secure = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) != "development";
                
                // Access token cookie
                let access_cookie = Cookie::build("access_token", token_pair.access_token.clone())
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .same_site(SameSite::Strict)
                    .max_age(time::Duration::minutes(30))
                    .finish();
                
                // Refresh token cookie
                let refresh_cookie = Cookie::build("refresh_token", token_pair.refresh_token.clone())
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .same_site(SameSite::Strict)
                    .max_age(time::Duration::days(7))
                    .finish();
                
                // Generate CSRF token
                let csrf_token = generate_csrf_token();
                let csrf_cookie = Cookie::build("csrf_token", csrf_token.token.clone())
                    .path("/")
                    .domain(&domain)
                    .http_only(false) // Must be accessible from JavaScript
                    .secure(secure)
                    .same_site(SameSite::Strict)
                    .max_age(time::Duration::days(7))
                    .finish();
                
                response.cookie(access_cookie).cookie(refresh_cookie).cookie(csrf_cookie).json(json!({
                    "success": true,
                    "message": "Login successful",
                    "data": {
                        "access_token": token_pair.access_token,
                        "refresh_token": token_pair.refresh_token,
                        "user": {
                            "id": user.id,
                            "username": user.username,
                            "email": user.email,
                            "is_email_verified": user.is_email_verified,
                            "created_at": user.created_at,
                            "role": user.role,
                            "is_active": user.is_active,
                            "csrf_token": csrf_token.token // Include CSRF token in response for frontend to use
                        }
                    }
                }))
            },
            Err(e) => {
                match e.to_string().as_str() {
                    "Email not verified" => {
                        HttpResponse::Unauthorized().json(json!({
                            "success": false,
                            "message": "Email has not been verified yet. Please check your email for the verification link.",
                            "error_type": "email_not_verified"
                        }))
                    },
                    _ => {
                        error!("Login error: {:?}", e);
                        HttpResponse::Unauthorized().json(json!({
                            "success": false,
                            "message": "Invalid username or password",
                            "error": "Invalid credentials"
                        }))
                    }
                }
            }
        }
    }

    pub async fn verify_email(
        &self,
        token_query: web::Query<TokenQuery>,
    ) -> Result<impl Responder, actix_web::Error> {
        let token = TokenQuery::try_from(token_query)?;
        debug!("Processing email verification token");

        match self.user_service.verify_email(token.token()).await {
            Ok(()) => {
                info!("Email verified successfully");
                Ok(HttpResponse::Found()
                    .append_header((header::LOCATION, "/email_verified"))
                    .finish())
            },
            Err(e) => {
                error!("Email verification failed: {:?}", e);
                Ok(HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": "The verification link is invalid or has expired",
                    "error": "Invalid verification token"
                })))
            }
        }
    }

    pub async fn get_user(
        &self,
        user_id: web::Path<Uuid>,
        auth: BearerAuth,
    ) -> impl Responder {
        match self.auth_service.validate_auth(auth.token()).await {
            Ok(claims) => {
                if claims.sub != *user_id {
                    warn!("Unauthorized access attempt: User {} tried to access data for user {}", claims.sub, user_id);
                    return HttpResponse::Forbidden().json(json!({
                        "success": false,
                        "message": "Access denied: You can only view your own information",
                        "error": "Unauthorized access"
                    }));
                }

                match self.user_service.get_user_by_id(*user_id).await {
                    Ok(user) => HttpResponse::Ok().json(json!({
                        "success": true,
                        "data": {
                            "user": {
                                "id": user.id,
                                "username": user.username,
                                "email": user.email,
                                "is_email_verified": user.is_email_verified,
                                "created_at": user.created_at,
                                "role": user.role,
                                "is_active": user.is_active
                            }
                        }
                    })),
                    Err(e) => {
                        error!("Failed to fetch user: {:?}", e);
                        HttpResponse::NotFound().json(json!({
                            "success": false,
                            "message": "User not found",
                            "error": "Not found"
                        }))
                    }
                }
            },
            Err(e) => {
                warn!("Invalid token: {:?}", e);
                HttpResponse::Unauthorized().json(json!({
                    "success": false,
                    "message": "Invalid token",
                    "error": "Authentication failed"
                }))
            }
        }
    }

    pub async fn get_current_user(
        &self,
        auth: BearerAuth,
    ) -> impl Responder {
        match self.auth_service.validate_auth(auth.token()).await {
            Ok(claims) => {
                match self.user_service.get_user_by_id(claims.sub).await {
                    Ok(user) => HttpResponse::Ok().json(json!({
                        "success": true,
                        "data": {
                            "user": {
                                "id": user.id,
                                "username": user.username,
                                "email": user.email,
                                "is_email_verified": user.is_email_verified,
                                "created_at": user.created_at,
                                "role": user.role,
                                "is_active": user.is_active
                            }
                        }
                    })),
                    Err(e) => {
                        error!("Failed to fetch current user: {:?}", e);
                        HttpResponse::NotFound().json(json!({
                            "success": false,
                            "message": "User not found",
                            "error": "Not found"
                        }))
                    }
                }
            },
            Err(e) => {
                warn!("Invalid token: {:?}", e);
                HttpResponse::Unauthorized().json(json!({
                    "success": false,
                    "message": "Invalid token",
                    "error": "Authentication failed"
                }))
            }
        }
    }

    pub async fn get_current_user_from_cookie(
        &self,
        req: HttpRequest,
    ) -> impl Responder {
        // Get claims from request extensions (added by CookieAuthMiddleware)
        if let Some(claims) = req.extensions().get::<Claims>() {
            debug!("Found claims in request extensions: {:?}", claims);
   
       
            match self.user_service.get_user_by_id(claims.sub).await {
                Ok(user) => HttpResponse::Ok().json(json!({
                    "success": true,
                    "data": {
                        "user": {
                            "id": user.id,
                            "username": user.username,
                            "email": user.email,
                            "is_email_verified": user.is_email_verified,
                            "created_at": user.created_at,
                            "role": user.role,
                            "is_active": user.is_active
                        }
                    },
                    "csrf_token": req.cookie("csrf_token").map(|c| c.value().to_string())
                })),
                Err(e) => {
                    error!("Failed to fetch current user from cookie: {:?}", e);
                    HttpResponse::NotFound().json(json!({
                        "success": false,
                        "message": "User not found",
                        "error": "Not found"
                    }))
                }
            }
        } else {
            HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "No access token cookie found",
                "error": "Missing authentication"
            }))
        }
    }

    pub async fn update_user(
        &self,
        user_id: web::Path<Uuid>,
        user_input: web::Json<UserInput>,
        auth: BearerAuth,
    ) -> impl Responder {
        match self.auth_service.validate_auth(auth.token()).await {
            Ok(claims) => {
                if claims.sub != *user_id {
                    return HttpResponse::Forbidden().json(json!({
                        "success": false,
                        "message": "Access denied: You can only update your own information",
                        "error": "Unauthorized access"
                    }));
                }

                match self.user_service.update_user(*user_id, user_input.into_inner()).await {
                    Ok(updated_user) => HttpResponse::Ok().json(json!({
                        "success": true,
                        "message": "User updated successfully",
                        "data": {
                            "user": {
                                "id": updated_user.id,
                                "username": updated_user.username,
                                "email": updated_user.email,
                                "is_email_verified": updated_user.is_email_verified,
                                "created_at": updated_user.created_at,
                                "role": updated_user.role,
                                "is_active": updated_user.is_active
                            }
                        }
                    })),
                    Err(e) => {
                        error!("Failed to update user {}: {:?}", user_id, e);
                        match e.error_type {
                            ApiErrorType::NotFound => HttpResponse::NotFound().json(json!({
                                "success": false,
                                "message": e.message,
                                "error": "User not found"
                            })),
                            ApiErrorType::Validation => HttpResponse::BadRequest().json(json!({
                                "success": false,
                                "message": e.message,
                                "error": "Validation failed",
                                "message": e.message
                            })),
                            ApiErrorType::Validation => HttpResponse::BadRequest().json(json!({
                                "success": false,
                                "message": e.message,
                                "error": "Conflict with existing resource"
                            })),
                            ApiErrorType::Authentication => HttpResponse::Unauthorized().json(json!({
                                "success": false,
                                "message": e.message,
                                "error": "Unauthorized"
                            })),
                            ApiErrorType::Authorization => HttpResponse::Forbidden().json(json!({
                                "success": false,
                                "message": e.message,
                                "error": "Forbidden"
                            })),
                            _ => HttpResponse::InternalServerError().json(json!({
                                "success": false,
                                "message": "An unexpected error occurred",
                                "error": "Internal server error"
                            })),
                        }
                    }
                }
            },
            Err(_) => HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "Invalid token",
                "error": "Authentication failed"
            }))
        }
    }

    pub async fn request_password_reset(
        &self,
        request: web::Json<PasswordResetRequest>,
    ) -> impl Responder {
        debug!("Received password reset request");

        match self.user_service.request_password_reset(&request.email).await {
            Ok(()) => {
                info!("Password reset email sent successfully");
                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "If an account exists with that email, you will receive password reset instructions."
                }))
            },
            Err(e) => {
                error!("Password reset request failed: {:?}", e);
                // Return success even on error to prevent email enumeration
                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "If an account exists with that email, you will receive password reset instructions."
                }))
            }
        }
    }

    pub async fn verify_reset_token(
        &self,
        token_query: web::Query<TokenQuery>,
    ) -> Result<impl Responder, actix_web::Error> {
        let token = TokenQuery::try_from(token_query)?;
        debug!("Verifying password reset token");

        match self.user_service.verify_reset_token(token.token()).await {
            Ok(()) => {
                info!("Password reset token verified successfully");
                Ok(HttpResponse::Found()
                    .append_header((header::LOCATION, format!("/password-reset/new?token={}", token.token())))
                    .finish())
            },
            Err(e) => {
                error!("Password reset token verification failed: {:?}", e);
                Ok(HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": "The password reset link is invalid or has expired",
                    "error": "Invalid reset token"
                })))
            }
        }
    }

    pub async fn reset_password(
        &self,
        reset_data: web::Json<PasswordResetSubmit>,
    ) -> impl Responder {
        debug!("Processing password reset");

        if reset_data.new_password != reset_data.confirm_password {
            return HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": "Passwords do not match",
                "error": "Password mismatch"
            }));
        }

        match self.user_service.reset_password(&reset_data.token, &reset_data.new_password).await {
            Ok(()) => {
                info!("Password reset successful");
                HttpResponse::Ok().json(json!({
                    "success": true,
                    "message": "Password has been reset successfully. You can now log in with your new password."
                }))
            },
            Err(e) => {
                error!("Password reset failed: {:?}", e);
                HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "message": e.to_string(),
                    "error": "Password reset failed"
                }))
            }
        }
    }

    pub async fn delete_user(
        &self,
        user_id: web::Path<Uuid>,
        auth: BearerAuth,
    ) -> impl Responder {
        match self.auth_service.validate_auth(auth.token()).await {
            Ok(claims) => {
                if claims.sub != *user_id {
                    return HttpResponse::Forbidden().json(json!({
                        "success": false,
                        "message": "Access denied: You can only delete your own account",
                        "error": "Unauthorized access"
                    }));
                }

                match self.user_service.delete_user(*user_id).await {
                    Ok(()) => HttpResponse::Ok().json(json!({
                        "success": true,
                        "message": "User deleted successfully"
                    })),
                    Err(e) => {
                        error!("Failed to delete user: {:?}", e);
                        HttpResponse::InternalServerError().json(json!({
                            "success": false,
                            "message": "Failed to delete user",
                            "error": "Internal server error"
                        }))
                    }
                }
            },
            Err(_) => HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "Invalid token",
                "error": "Authentication failed"
            }))
        }
    }
    
    /// Logout a user by revoking their tokens
    /// Refresh an access token using a refresh token
    pub async fn refresh_token(
        &self,
        refresh_token: web::Json<RefreshToken>,
    ) -> impl Responder {
        debug!("Attempting to refresh access token from JSON body");
        
        match self.auth_service.refresh_token(&refresh_token.token).await {
            Ok(token_pair) => {
                info!("Tokens refreshed successfully");
                
                // Create response
                let mut response = HttpResponse::Ok();
                
                // Set cookies
                let domain = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let secure = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) != "development";
                
                // Access token cookie
                let access_cookie = Cookie::build("access_token", token_pair.access_token.clone())
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .same_site(SameSite::Strict)
                    .max_age(time::Duration::minutes(30))
                    .finish();
                
                // Refresh token cookie
                let refresh_cookie = Cookie::build("refresh_token", token_pair.refresh_token.clone())
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .same_site(SameSite::Strict)
                    .max_age(time::Duration::days(7))
                    .finish();
                
                // Generate CSRF token
                let csrf_token = generate_csrf_token();
                let csrf_cookie = Cookie::build("csrf_token", csrf_token.token.clone())
                    .path("/")
                    .domain(&domain)
                    .http_only(false) // Must be accessible from JavaScript
                    .secure(secure)
                    .same_site(SameSite::Strict)
                    .max_age(time::Duration::days(7))
                    .finish();
                
                response.cookie(access_cookie).cookie(refresh_cookie).cookie(csrf_cookie).json(json!({
                    "success": true,
                    "message": "Token refreshed successfully",
                    "data": {
                        "access_token": token_pair.access_token,
 // Still include in response for backward compatibility
                        "refresh_token": token_pair.refresh_token
                    }
,
                    "csrf_token": csrf_token.token
                }))
            },
            Err(e) => {
                error!("Failed to refresh token: {:?}", e);
                HttpResponse::Unauthorized().json(json!({
                    "success": false,
                    "message": "Invalid refresh token",
                    "error": "Authentication failed"
                }))
            }
        }
    }
    
    /// Refresh token using cookie
    pub async fn refresh_token_from_cookie(
        &self,
        req: HttpRequest,
    ) -> impl Responder {
        // Get refresh token from cookie
        if let Some(cookie) = req.cookie("refresh_token") {
            debug!("Attempting to refresh access token from cookie");
            
            match self.auth_service.refresh_token(cookie.value()).await {
                Ok(token_pair) => {
                    info!("Tokens refreshed successfully from cookie");
                    
                    // Create response with new cookies (similar to refresh_token method)
                    let mut response = HttpResponse::Ok();
                    
                    // Set cookies (same as in refresh_token method)
                    let domain = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                    let secure = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) != "development";
                    
                    // Set cookies (same as in refresh_token method)
                    let access_cookie = Cookie::build("access_token", token_pair.access_token)
                        .path("/").domain(&domain).http_only(true).secure(secure)
                        .same_site(SameSite::Strict).max_age(time::Duration::minutes(30)).finish();
                    
                    let refresh_cookie = Cookie::build("refresh_token", token_pair.refresh_token)
                        .path("/").domain(&domain).http_only(true).secure(secure)
                        .same_site(SameSite::Strict).max_age(time::Duration::days(7)).finish();

                    // Generate CSRF token
                    let csrf_token = generate_csrf_token();
                    let csrf_cookie = Cookie::build("csrf_token", csrf_token.token.clone())
                        .path("/")
                        .domain(&domain)
                        .http_only(false) // Must be accessible from JavaScript
                        .secure(secure)
                        .same_site(SameSite::Strict)
                        .max_age(time::Duration::days(7))
                        .finish();
                        
                    response.cookie(access_cookie).cookie(refresh_cookie).cookie(csrf_cookie).json(json!({
                        "success": true, 
                        "message": "Token refreshed successfully",
                        "csrf_token": csrf_token.token
                    }))
                },
                Err(e) => HttpResponse::Unauthorized().json(json!({"success": false, "message": "Invalid refresh token", "error": e.to_string()}))
            }
        } else {
            HttpResponse::Unauthorized().json(json!({"success": false, "message": "No refresh token cookie found", "error": "Missing refresh token"}))
        }
    }
    
    pub async fn logout_user(
        &self,
        auth: BearerAuth,
        refresh_token: Option<web::Json<RefreshToken>>,
    ) -> impl Responder {
        // Get the access token from the auth header
        let access_token = auth.token();
        
        // Get the refresh token from the request body if provided
        let refresh_token = refresh_token.map(|rt| rt.into_inner().token);
        
        // Logout the user by revoking their tokens
        match self.auth_service.logout(access_token, refresh_token.as_deref()).await {
            Ok(()) => {
                // Create response
                let mut response = HttpResponse::Ok();
                
                // Set cookies with immediate expiration to clear them
                let domain = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let secure = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) != "development";
                
                // Clear access token cookie
                let access_cookie = Cookie::build("access_token", "")
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .max_age(time::Duration::seconds(0))
                    .finish();
                
                // Clear refresh token cookie
                let refresh_cookie = Cookie::build("refresh_token", "")
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .max_age(time::Duration::seconds(0))
                    .finish();
                
                // Clear CSRF token cookie
                let csrf_cookie = Cookie::build("csrf_token", "")
                    .path("/")
                    .domain(&domain)
                    .http_only(false)
                    .secure(secure)
                    .max_age(time::Duration::seconds(0))
                    .finish();
                
                info!("User logged out successfully");
                response.cookie(access_cookie).cookie(refresh_cookie).cookie(csrf_cookie).json(json!({
                    "success": true,
                    "message": "Logged out successfully"
                }))
            },
            Err(e) => {
                error!("Failed to logout: {:?}", e);
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "message": "Failed to logout",
                    "error": "Internal server error"
                }))
            }
        }
    }
    
    pub async fn logout_user_from_cookie(
        &self,
        req: HttpRequest,
        auth: Option<BearerAuth>,
    ) -> impl Responder {
        // Try to get access token from cookie first
        let access_token = if let Some(cookie) = req.cookie("access_token") {
            cookie.value().to_string()
        } else if let Some(auth) = auth {
            // Fallback to Authorization header for backward compatibility
            auth.token().to_string()
        } else {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "No authentication token found",
                "error": "Missing access token"
            }));
        };
        
        // Get refresh token from cookie if available
        let refresh_token = req.cookie("refresh_token").map(|c| c.value().to_string());
        
        // Logout the user by revoking their tokens
        match self.auth_service.logout(&access_token, refresh_token.as_deref()).await {
            Ok(()) => {
                // Create response
                let mut response = HttpResponse::Ok();
                
                // Set cookies with immediate expiration to clear them
                let domain = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
                let secure = std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) != "development";
                
                // Clear access token cookie
                let access_cookie = Cookie::build("access_token", "")
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .max_age(time::Duration::seconds(0))
                    .finish();
                
                // Clear refresh token cookie
                let refresh_cookie = Cookie::build("refresh_token", "")
                    .path("/")
                    .domain(&domain)
                    .http_only(true)
                    .secure(secure)
                    .max_age(time::Duration::seconds(0))
                    .finish();
                
                // Clear CSRF token cookie
                let csrf_cookie = Cookie::build("csrf_token", "")
                    .path("/")
                    .domain(&domain)
                    .http_only(false)
                    .secure(secure)
                    .max_age(time::Duration::seconds(0))
                    .finish();
                
                info!("User logged out successfully");
                response.cookie(access_cookie).cookie(refresh_cookie).cookie(csrf_cookie).json(json!({
                    "success": true,
                    "message": "Logged out successfully"
                }))
            },
            Err(e) => {
                error!("Failed to logout: {:?}", e);
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "message": "Failed to logout",
                    "error": "Internal server error"
                }))
            }
        }
    }
}

// Factory function to create handler instance
pub fn create_handler(
    pool: PgPool,
    email_service: Arc<dyn EmailServiceTrait>,
    auth_service: Arc<AuthService>,
    token_revocation_service: Arc<dyn TokenRevocationServiceTrait>,
    active_token_service: Arc<dyn ActiveTokenServiceTrait>
) -> UserHandler {
    UserHandler::new(pool, email_service, auth_service, token_revocation_service, active_token_service)
}

// Route handler functions
pub async fn create_user_handler(
    handler: web::Data<UserHandler>,
    register_input: web::Json<RegisterInput>,
) -> impl Responder {
    handler.create_user(register_input).await
}

pub async fn login_user_handler(
    handler: web::Data<UserHandler>,
    login_input: web::Json<LoginInput>,
) -> impl Responder {
    handler.login_user(login_input).await
}
 

pub async fn verify_email_handler(
    handler: web::Data<UserHandler>,
    token_query: web::Query<TokenQuery>,
) -> Result<impl Responder, actix_web::Error> {
    handler.verify_email(token_query).await
}

pub async fn get_user_handler(
    handler: web::Data<UserHandler>,
    user_id: web::Path<Uuid>,
    auth: BearerAuth,
) -> impl Responder {
    handler.get_user(user_id, auth).await
}

pub async fn get_current_user_handler(
    handler: web::Data<UserHandler>,
    auth: BearerAuth,
) -> impl Responder {
    handler.get_current_user(auth).await
}

pub async fn get_current_user_from_cookie_handler(
    handler: web::Data<UserHandler>,
    req: HttpRequest,
) -> impl Responder {
    handler.get_current_user_from_cookie(req).await
}

pub async fn update_user_handler(
    handler: web::Data<UserHandler>,
    user_id: web::Path<Uuid>,
    user_input: web::Json<UserInput>,
    auth: BearerAuth,
) -> impl Responder {
    handler.update_user(user_id, user_input, auth).await
}

pub async fn request_password_reset_handler(
    handler: web::Data<UserHandler>,
    request: web::Json<PasswordResetRequest>,
) -> impl Responder {
    handler.request_password_reset(request).await
}

pub async fn verify_reset_token_handler(
    handler: web::Data<UserHandler>,
    token_query: web::Query<TokenQuery>,
) -> Result<impl Responder, actix_web::Error> {
    handler.verify_reset_token(token_query).await
}

pub async fn reset_password_handler(
    handler: web::Data<UserHandler>,
    reset_data: web::Json<PasswordResetSubmit>,
) -> impl Responder {
    handler.reset_password(reset_data).await
}

pub async fn delete_user_handler(
    handler: web::Data<UserHandler>,
    user_id: web::Path<Uuid>,
    auth: BearerAuth,
) -> impl Responder {
    handler.delete_user(user_id, auth).await
}

pub async fn logout_user_handler(
    handler: web::Data<UserHandler>,
    req: HttpRequest,
    auth: Option<BearerAuth>,
) -> impl Responder {
    handler.as_ref().logout_user_from_cookie(req, auth).await
}

pub async fn refresh_token_handler(
    handler: web::Data<UserHandler>,
    refresh_token: web::Json<RefreshToken>,
) -> impl Responder {
    handler.refresh_token(refresh_token).await
}
 

pub async fn refresh_token_from_cookie_handler(
    handler: web::Data<UserHandler>,
    req: HttpRequest,
) -> impl Responder {
    handler.refresh_token_from_cookie(req).await
}
