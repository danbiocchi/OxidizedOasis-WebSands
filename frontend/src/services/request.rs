use gloo::console::log;
use gloo::net::http::{Request, RequestBuilder, Response};
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::pin::Pin;
use crate::services::auth;

/// RequestInterceptor provides a wrapper around the gloo::net::http::Request
/// with automatic token refresh and retry capabilities.
pub struct RequestInterceptor;

impl RequestInterceptor {
    #[allow(dead_code)]
    /// Creates a GET request with automatic token refresh and retry capabilities
    pub fn get(url: &str) -> RequestBuilder {
        let mut request = Request::get(url);
        
        // Add CSRF token if available
        if let Some(csrf_token) = auth::get_csrf_token() {
            request = request.header("X-CSRF-Token", &csrf_token);
        }
        
        // Check if token is about to expire and refresh proactively
        Self::check_token_expiration();
        
        request
    }

    #[allow(dead_code)]
    /// Creates a POST request with automatic token refresh and retry capabilities
    pub fn post(url: &str) -> RequestBuilder {
        let mut request = Request::post(url);
        
        // Add CSRF token if available
        if let Some(csrf_token) = auth::get_csrf_token() {
            request = request.header("X-CSRF-Token", &csrf_token);
        }
        
        // Check if token is about to expire and refresh proactively
        Self::check_token_expiration();
        
        request
    }

    #[allow(dead_code)]
    /// Creates a PUT request with automatic token refresh and retry capabilities
    pub fn put(url: &str) -> RequestBuilder {
        let mut request = Request::put(url);
        
        // Add CSRF token if available
        if let Some(csrf_token) = auth::get_csrf_token() {
            request = request.header("X-CSRF-Token", &csrf_token);
        }
        
        // Check if token is about to expire and refresh proactively
        Self::check_token_expiration();
        
        request
    }

    #[allow(dead_code)]
    /// Creates a DELETE request with automatic token refresh and retry capabilities
    pub fn delete(url: &str) -> RequestBuilder {
        let mut request = Request::delete(url);
        
        // Add CSRF token if available
        if let Some(csrf_token) = auth::get_csrf_token() {
            request = request.header("X-CSRF-Token", &csrf_token);
        }
        
        // Check if token is about to expire and refresh proactively
        Self::check_token_expiration();
        
        request
    }
    
    /// Check if token is about to expire and refresh proactively
    #[allow(dead_code)]
    fn check_token_expiration() {
        // This will trigger the token refresh timer setup if needed
        auth::setup_token_refresh_timer();
    }
}

/// Extension trait for RequestBuilder and Request to add send_with_retry method
pub trait RequestBuilderExt {
    fn send_with_retry(self) -> Pin<Box<dyn Future<Output = Result<Response, String>>>>;
}

impl RequestBuilderExt for Request {
    fn send_with_retry(self) -> Pin<Box<dyn Future<Output = Result<Response, String>>>> {
        Box::pin(async move {
            gloo::console::log!("🔍 DEBUG: RequestInterceptor Request Attempt");
            gloo::console::log!("🔍 DEBUG: Request URL:", &self.url().to_string());
            gloo::console::log!("🔍 DEBUG: Request Method:", &self.method().as_str().to_string());
            
            // Check if we have auth tokens before sending
            let has_access_token = crate::services::auth::get_auth_token().is_some();
            let has_csrf_token = crate::services::auth::get_csrf_token().is_some();
            gloo::console::log!("🔍 DEBUG: Has access token:", has_access_token);
            gloo::console::log!("🔍 DEBUG: Has CSRF token:", has_csrf_token);
            
            // First attempt
            let response = self.send().await.map_err(|e| {
                gloo::console::log!("🔍 DEBUG: Request failed with error:", &e.to_string());
                e.to_string()
            })?;
            
            gloo::console::log!("🔍 DEBUG: Response status:", response.status());
            gloo::console::log!("🔍 DEBUG: Response headers:", format!("{:?}", response.headers()));
            
            // If unauthorized, try to refresh token and retry
            if response.status() == 401 || response.status() == 403 {
                gloo::console::log!("RequestInterceptor: Got 401/403, attempting token refresh");
                log!("Request failed with status: {}, attempting token refresh", response.status());
                
                // Try to refresh the token
                match auth::refresh_access_token().await {
                    Ok(()) => {
                        log!("Token refreshed successfully, retrying request");
                        
                        // We need to recreate the request since we can't clone Request
                        // Get the URL from the response
                        let url = response.url().to_string();
                        log!("Retrying request to URL: {}", &url);
                        
                        // Extract the HTTP method from the URL path
                        let method = determine_http_method(&url);
                        log!("Determined HTTP method: {}", method);
                        
                        // Create a new request with the determined method
                        let mut new_request = match method {
                            "GET" => Request::get(&url),
                            "POST" => Request::post(&url),
                            "PUT" => Request::put(&url),
                            "DELETE" => Request::delete(&url),
                            _ => {
                                log!("Unknown method, defaulting to GET");
                                Request::get(&url)
                            }
                        };
                        
                        // Add the new CSRF token
                        if let Some(csrf_token) = auth::get_csrf_token() {
                            new_request = new_request.header("X-CSRF-Token", &csrf_token);
                        }
                        
                        // Send the new request
                        let new_response = new_request.send().await.map_err(|e| e.to_string())?;
                        
                        // If still unauthorized after refresh, return error
                        if new_response.status() == 401 || new_response.status() == 403 {
                            return Err("Still unauthorized after token refresh".to_string());
                        }
                        
                        Ok(new_response)
                    },
                    Err(e) => {
                        // Token refresh failed, likely need to re-login
                        auth::remove_tokens();
                        Err(format!("Token refresh failed: {}", e))
                    }
                }
            } else {
                // Return the original response if not unauthorized
                Ok(response)
            }
        })
    }
}

impl RequestBuilderExt for RequestBuilder {
    fn send_with_retry(self) -> Pin<Box<dyn Future<Output = Result<Response, String>>>> {
        Box::pin(async move {
            gloo::console::log!("🔍 DEBUG: RequestBuilder Request Attempt");
            
            // Check if we have auth tokens before sending
            let has_access_token = crate::services::auth::get_auth_token().is_some();
            let has_csrf_token = crate::services::auth::get_csrf_token().is_some();
            gloo::console::log!("🔍 DEBUG: Has access token:", has_access_token);
            gloo::console::log!("🔍 DEBUG: Has CSRF token:", has_csrf_token);
            
            // First attempt
            let response = self.send().await.map_err(|e| {
                gloo::console::log!("🔍 DEBUG: RequestBuilder failed with error:", &e.to_string());
                e.to_string()
            })?;
            
            gloo::console::log!("🔍 DEBUG: RequestBuilder Response status:", response.status());
            gloo::console::log!("�� DEBUG: RequestBuilder Response headers:", format!("{:?}", response.headers()));
            
            // If unauthorized, try to refresh token and retry
            if response.status() == 401 || response.status() == 403 {
                gloo::console::log!("🔍 DEBUG: Got 401/403, attempting token refresh");
                log!("🔍 DEBUG: Request failed with status: {}, attempting token refresh", response.status());
                
                // Try to refresh the token
                match auth::refresh_access_token().await {
                    Ok(()) => {
                        log!("Token refreshed successfully, retrying request");
                        
                        // We need to recreate the request since we can't clone RequestBuilder
                        // Get the URL from the response
                        let url = response.url().to_string();
                        log!("Retrying request to URL: {}", &url);
                        
                        // Extract the HTTP method from the URL path
                        let method = determine_http_method(&url);
                        log!("Determined HTTP method: {}", method);
                        
                        // Create a new request with the determined method
                        let mut new_request = match method {
                            "GET" => Request::get(&url),
                            "POST" => Request::post(&url),
                            "PUT" => Request::put(&url),
                            "DELETE" => Request::delete(&url),
                            _ => {
                                log!("Unknown method, defaulting to GET");
                                Request::get(&url)
                            }
                        };
                        
                        // Add the new CSRF token
                        if let Some(csrf_token) = auth::get_csrf_token() {
                            new_request = new_request.header("X-CSRF-Token", &csrf_token);
                        }
                        
                        // Send the new request
                        let new_response = new_request.send().await.map_err(|e| e.to_string())?;
                        
                        // If still unauthorized after refresh, return error
                        if new_response.status() == 401 || new_response.status() == 403 {
                            return Err("Still unauthorized after token refresh".to_string());
                        }
                        
                        Ok(new_response)
                    },
                    Err(e) => {
                        // Token refresh failed, likely need to re-login
                        auth::remove_tokens();
                        Err(format!("Token refresh failed: {}", e))
                    }
                }
            } else {
                // Return the original response if not unauthorized
                Ok(response)
            }
        })
    }
}

// Helper function to determine HTTP method based on URL path
fn determine_http_method(url: &str) -> &'static str {
    // Extract the path part of the URL
    let path = url.split('?').next().unwrap_or(url);
    
    // Check for specific patterns in the URL to determine the HTTP method
    if path.contains("/users/logout") {
        "POST"
    } else if path.contains("/users/refresh") {
        "GET"
    } else if path.contains("/admin/users/") {
        // Admin user management endpoints
        if path.contains("/role") || path.contains("/status") || path.contains("/username") {
            "PUT"
        } else if path.ends_with("/users") {
            // List users endpoint
            "GET"
        } else {
            // User detail endpoint - could be GET or PUT
            // Default to GET for safety
            "GET"
        }
    } else if path.contains("edit") || path.contains("update") {
        // Edit/update operations are typically PUT
        "PUT"
    } else if path.contains("delete") || path.contains("remove") {
        // Delete operations
        "DELETE"
    } else if path.contains("create") || path.contains("add") || path.contains("login") || path.contains("register") {
        // Create/add operations are typically POST
        "POST"
    } else {
        // Default to GET for other endpoints
        "GET"
    }
}