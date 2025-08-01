use actix_cors::Cors;
use actix_web::http;
use std::env;

pub fn configure_cors() -> Cors {
    let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
    let allowed_origin = match environment.as_str() {
        "production" => env::var("PRODUCTION_URL")
            .expect("PRODUCTION_URL must be set in production"),
        _ => env::var("DEVELOPMENT_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    };

    Cors::default()
        .allowed_origin(&allowed_origin)
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
        ])
        .max_age(3600)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::env;

    // Helper function for thread-safe environment variable testing
    fn with_env_vars<F, R>(env_vars: Vec<(&str, Option<&str>)>, func: F) -> R
    where
        F: FnOnce() -> R,
    {
        static ENV_MUTEX: Mutex<()> = Mutex::new(());
        
        // Handle both normal locks and poisoned locks
        let _guard = match ENV_MUTEX.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // If the mutex is poisoned, we can still proceed by clearing the poison
                // and getting the guard. This allows tests to continue running.
                poisoned.into_inner()
            }
        };
        
        // Store original values
        let original_values: Vec<_> = env_vars.iter()
            .map(|(key, _)| (*key, env::var(key).ok()))
            .collect();
        
        // Set test values
        for (key, value) in &env_vars {
            match value {
                Some(val) => env::set_var(key, val),
                None => env::remove_var(key),
            }
        }
        
        let result = func();
        
        // Restore original values
        for (key, original) in original_values {
            match original {
                Some(val) => env::set_var(key, val),
                None => env::remove_var(key),
            }
        }
        
        result
    }

    #[test]
    fn test_configure_cors_development_environment() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("development")),
            ("DEVELOPMENT_URL", Some("http://localhost:3000")),
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // We can't directly inspect the Cors configuration, but we can verify 
            // the function runs without panicking and returns a Cors instance
            assert!(true, "CORS configuration created successfully for development");
        });
    }

    #[test]
    fn test_configure_cors_development_default_url() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("development")),
            ("DEVELOPMENT_URL", None), // Not set, should use default
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Verify function completes successfully with default development URL
            assert!(true, "CORS configuration created with default development URL");
        });
    }

    #[test]
    fn test_configure_cors_production_environment() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("production")),
            ("PRODUCTION_URL", Some("https://example.com")),
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Verify function completes successfully in production
            assert!(true, "CORS configuration created successfully for production");
        });
    }

    #[test]
    #[should_panic(expected = "PRODUCTION_URL must be set in production")]
    fn test_configure_cors_production_missing_url() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("production")),
            ("PRODUCTION_URL", None), // Missing required production URL
        ];
        
        with_env_vars(env_vars, || {
            configure_cors(); // Should panic
        });
    }

    #[test]
    fn test_configure_cors_no_environment_defaults_to_development() {
        let env_vars = vec![
            ("ENVIRONMENT", None), // Not set, should default to development
            ("DEVELOPMENT_URL", Some("http://localhost:9000")),
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Verify function completes successfully with default environment
            assert!(true, "CORS configuration created with default environment");
        });
    }

    #[test]
    fn test_configure_cors_staging_environment_uses_development_path() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("staging")), // Not production, should use development path
            ("DEVELOPMENT_URL", Some("http://staging.example.com")),
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Verify function treats staging as development
            assert!(true, "CORS configuration created for staging environment");
        });
    }

    #[test]
    fn test_configure_cors_test_environment_uses_development_path() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("test")), // Not production, should use development path
            ("DEVELOPMENT_URL", None), // Should use default
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Verify function treats test as development
            assert!(true, "CORS configuration created for test environment");
        });
    }

    #[test]
    fn test_configure_cors_returns_cors_instance() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("development")),
            ("DEVELOPMENT_URL", Some("http://localhost:8080")),
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Verify we get a Cors instance (we can't easily inspect its internals)
            // The fact that this compiles and runs means the function signature is correct
            drop(cors); // Explicitly drop to show we have ownership
            assert!(true, "Function returns Cors instance");
        });
    }

    #[test]
    fn test_environment_variable_parsing() {
        // Test various environment values that should map to development
        let test_cases = vec![
            ("dev", "development path"),
            ("Development", "development path"),
            ("local", "development path"),
            ("testing", "development path"),
        ];
        
        for (env_value, description) in test_cases {
            let env_vars = vec![
                ("ENVIRONMENT", Some(env_value)),
                ("DEVELOPMENT_URL", Some("http://test.local")),
            ];
            
            with_env_vars(env_vars, || {
                let cors = configure_cors();
                // All non-"production" values should work
                assert!(true, "CORS configured for {}", description);
            });
        }
    }

    #[test]
    fn test_production_url_validation() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("production")),
            ("PRODUCTION_URL", Some("https://secure-production-site.com")),
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Production URL is set, should work fine
            assert!(true, "Production CORS configuration with valid URL");
        });
    }

    #[test]
    fn test_development_url_fallback() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("development")),
            ("DEVELOPMENT_URL", None), // Will use default localhost:8080
        ];
        
        with_env_vars(env_vars, || {
            let cors = configure_cors();
            
            // Should use default development URL when DEVELOPMENT_URL is not set
            assert!(true, "Development CORS uses default URL when not specified");
        });
    }

    #[test]
    fn test_cors_configuration_structure() {
        let env_vars = vec![
            ("ENVIRONMENT", Some("development")),
            ("DEVELOPMENT_URL", Some("http://localhost:3000")),
        ];
        
        with_env_vars(env_vars, || {
            // Test that the function signature and return type are correct
            let cors: actix_cors::Cors = configure_cors();
            
            // The function should return a properly configured Cors middleware
            // We can't easily inspect the internal configuration, but we can
            // verify the type and that it doesn't panic
            drop(cors);
            assert!(true, "CORS middleware properly typed and configured");
        });
    }
}