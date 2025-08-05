use std::env;
use std::sync::Arc;
use std::time::Duration;
use actix_files as fs;
use actix_web::{App, HttpResponse, HttpServer, web, middleware};
use dotenv::dotenv;
use env_logger::Env;
use log::{debug, error, info, warn};
use sqlx::postgres::Postgres;

// Corrected imports based on typical project structure and previous cargo fix hints
// Removed: use crate::api::routes::route_config::configure_all as configure_routes; 
use crate::api::handlers::user_handler::create_handler as create_user_handler_factory;
use crate::core::email::service::EmailService; 
use crate::core::user::{UserRepository, UserRepositoryTrait};
use crate::core::auth::AuthService;
use crate::core::auth::token_revocation::{TokenRevocationService, TokenRevocationServiceTrait};
use crate::core::auth::active_token::{ActiveTokenService, ActiveTokenServiceTrait};
use crate::infrastructure::config::AppConfig; 
use crate::infrastructure::middleware::cors::configure_cors; 
use crate::infrastructure::database::create_pool; // Use re-exported path
use crate::infrastructure::middleware::logger::RequestLogger; 
use crate::infrastructure::middleware::rate_limit::RateLimiter; 
use crate::infrastructure::middleware::metrics::RequestMetrics;

mod api;
mod common;
mod core;
mod infrastructure;

fn validate_critical_env_vars() -> Result<(), Box<dyn std::error::Error>> {
    let required_vars = [
        "DATABASE_URL",
        "JWT_SECRET",
        "SMTP_SERVER",
        "ADMIN_EMAIL",
    ];
    for var in required_vars {
        if env::var(var).is_err() {
            return Err(format!("Missing required environment variable: {var}").into());
        }
    }
    Ok(())
}

async fn setup_database(config: &AppConfig, run_migrations: bool) -> Result<sqlx::Pool<Postgres>, Box<dyn std::error::Error>> {
    let pool = create_pool(config).await?; // create_pool is now in scope
    if run_migrations {
        info!("Running database migrations");
        match sqlx::migrate!("./migrations").run(&pool).await {
            Ok(_) => info!("Migrations completed successfully"),
            Err(e) => {
                error!("Migration failed: {e:?}");
                return Err(Box::new(e));
            }
        }
    } else {
        warn!("Skipping database migrations. Ensure your database schema is up to date.");
    }
    Ok(pool)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    info!("Starting OxidizedOasis-WebSands application");

    if let Err(e) = validate_critical_env_vars() {
        error!("Environment validation failed: {e}");
        return Err(std::io::Error::other(e.to_string()));
    }

    let config = match AppConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {e:?}");
            return Err(std::io::Error::other(e.to_string()));
        }
    };

    let config_clone = config.clone();
    let server_host = config.server.host.clone();
    let server_port = config.server.port.clone();
    let run_migrations = env::var("RUN_MIGRATIONS")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(true);

    let pool = match tokio::time::timeout(
        Duration::from_secs(30),
        setup_database(&config, run_migrations)
    ).await {
        Ok(Ok(pool)) => pool,
        Ok(Err(e)) => {
            error!("Database setup failed: {e:?}");
            return Err(std::io::Error::other(e.to_string()));
        }
        Err(_) => {
            error!("Database setup timed out");
            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "Database setup timed out"));
        }
    };

    let email_service_arc = Arc::new(EmailService::new());
    let active_token_service_arc: Arc<dyn ActiveTokenServiceTrait> = Arc::new(ActiveTokenService::new(pool.clone()));
    
    let mut token_revocation_service_mut = TokenRevocationService::new(pool.clone());
    token_revocation_service_mut.set_active_token_service(active_token_service_arc.clone());
    let token_revocation_service_arc: Arc<dyn TokenRevocationServiceTrait> = Arc::new(token_revocation_service_mut);

    // Create the UserRepositoryTrait instance
    let user_repository_arc: Arc<dyn UserRepositoryTrait> = Arc::new(UserRepository::new(pool.clone()));
    
    let jwt_secret_main = env::var("JWT_SECRET").expect("JWT_SECRET must be set for main");
    let auth_service_arc = Arc::new(AuthService::new(
        user_repository_arc.clone(), // Use the Arc<dyn Trait>
        jwt_secret_main,
        config.jwt.audience.clone(), // Pass the JWT audience from AppConfig
        token_revocation_service_arc.clone(),
        active_token_service_arc.clone(),
        email_service_arc.clone() // Added email_service_arc
    ));
    
    let cleanup_revoked_service = token_revocation_service_arc.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); 
        loop {
            interval.tick().await;
            match cleanup_revoked_service.cleanup_expired_tokens().await {
                Ok(count) => {
                    if count > 0 { info!("Cleaned up {count} expired revoked tokens"); }
                },
                Err(e) => { error!("Failed to clean up expired revoked tokens: {e:?}"); }
            }
        }
    });

    let cleanup_active_service = active_token_service_arc.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); 
        loop {
            interval.tick().await;
            match cleanup_active_service.cleanup_expired_tokens().await {
                Ok(count) => {
                    if count > 0 { info!("Cleaned up {count} expired active tokens"); }
                },
                Err(e) => { error!("Failed to clean up expired active tokens: {e:?}"); }
            }
        }
    });
    
    let user_handler_data = web::Data::new(create_user_handler_factory(
        pool.clone(),
        email_service_arc.clone(),
        auth_service_arc.clone(), // Pass AuthService
        token_revocation_service_arc.clone(), // Pass TokenRevocationService
        active_token_service_arc.clone() // Pass ActiveTokenService
    ));

    let server_addr = format!("{server_host}:{server_port}");
    debug!("Server will be listening on: {server_addr}");

    HttpServer::new(move || {
        let app_config_clone = config_clone.clone(); 
        App::new()
            .wrap(RequestMetrics) // Added RequestMetrics middleware
            .wrap(RateLimiter::new()) // Assuming RateLimiter is correctly in scope via use statement
            .wrap(configure_cors()) // Assuming configure_cors is correctly in scope via use statement
            .wrap(
                middleware::DefaultHeaders::new()
                    .add(("X-XSS-Protection", "0"))
                    .add(("Strict-Transport-Security", "max-age=31536000; includeSubDomains"))
                    .add(("X-Frame-Options", "DENY"))
                    .add(("X-Content-Type-Options", "nosniff"))
                    .add(("Referrer-Policy", "strict-origin-when-cross-origin"))
                    .add(("Permissions-Policy", "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()"))
                    .add(("Cross-Origin-Embedder-Policy", "require-corp"))
                    .add(("Cross-Origin-Opener-Policy", "same-origin"))
                    .add(("Cross-Origin-Resource-Policy", "same-origin"))
                    .add(("Content-Security-Policy", "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval' https://cdn.jsdelivr.net https://cdnjs.cloudflare.com; style-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com; img-src 'self' data:; connect-src 'self' ws://127.0.0.1:* wss://127.0.0.1:*; font-src 'self' https://cdnjs.cloudflare.com; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none'; worker-src 'self' blob:; upgrade-insecure-requests;"))
            )
            .wrap(RequestLogger::new()) // Assuming RequestLogger is correctly in scope via use statement
            .wrap(middleware::Compress::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(email_service_arc.clone()))
            .app_data(user_handler_data.clone()) // Add UserHandler
            .app_data(web::Data::new(auth_service_arc.clone())) 
            .app_data(web::Data::new(token_revocation_service_arc.clone())) // Make TRS available
            .app_data(web::Data::new(active_token_service_arc.clone()))   // Make ATS available
            .app_data(web::Data::new(app_config_clone.clone())) 
            .app_data(web::Data::new(user_repository_arc.clone())) // Provide Arc<dyn UserRepositoryTrait>
            .app_data(web::JsonConfig::default()
                .limit(4096)
                .error_handler(|err, _| {
                    actix_web::error::InternalError::from_response(
                        err,
                        HttpResponse::BadRequest()
                            .content_type("application/json")
                            .body(r#"{"error":"Invalid JSON payload"}"#),
                    ).into()
                }))
            .service(fs::Files::new("/static/css", "./frontend/static/css").prefer_utf8(true).use_last_modified(true))
            .configure(crate::api::routes::route_config::configure_all) // Use full path
            .service(fs::Files::new("/", "./frontend/dist").index_file("index.html").prefer_utf8(true).use_last_modified(true))
            .default_service(web::route().to(|| async {
                match std::fs::read_to_string("./frontend/dist/index.html") {
                    Ok(contents) => HttpResponse::Ok().content_type("text/html; charset=utf-8").append_header(("Cache-Control", "no-store, must-revalidate")).append_header(("Pragma", "no-cache")).append_header(("Expires", "0")).body(contents),
                    Err(e) => {
                        error!("Failed to read index.html: {e}"); 
                        HttpResponse::InternalServerError().content_type("application/json").body(r#"{"error": "An unexpected error occurred"}"#) 
                    }
                }
            }))
    })
        .keep_alive(Duration::from_secs(75))
        .client_request_timeout(Duration::from_secs(60))
        .server_hostname(server_host)
        .backlog(1024)
        .workers(num_cpus::get() * 2)
        .shutdown_timeout(30)
        .bind(&server_addr)?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // Global mutex to prevent concurrent environment variable modifications in tests
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn setup_test_env_vars() {
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/test");
        env::set_var("JWT_SECRET", "test_jwt_secret_key_for_testing");
        env::set_var("SMTP_SERVER", "smtp.test.com");
        env::set_var("ADMIN_EMAIL", "admin@test.com");
    }

    fn cleanup_test_env_vars() {
        let vars_to_remove = ["DATABASE_URL", "JWT_SECRET", "SMTP_SERVER", "ADMIN_EMAIL"];
        for var in &vars_to_remove {
            env::remove_var(var);
        }
    }

    #[test]
    fn test_validate_critical_env_vars_success() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();
        setup_test_env_vars();

        let result = validate_critical_env_vars();
        assert!(result.is_ok(), "Should succeed when all env vars are set");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_missing_database_url() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();
        
        // Set all except DATABASE_URL
        env::set_var("JWT_SECRET", "test_jwt_secret");
        env::set_var("SMTP_SERVER", "smtp.test.com");
        env::set_var("ADMIN_EMAIL", "admin@test.com");

        let result = validate_critical_env_vars();
        assert!(result.is_err(), "Should fail when DATABASE_URL is missing");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("DATABASE_URL"), "Error should mention DATABASE_URL");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_missing_jwt_secret() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();
        
        // Set all except JWT_SECRET
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/test");
        env::set_var("SMTP_SERVER", "smtp.test.com");
        env::set_var("ADMIN_EMAIL", "admin@test.com");

        let result = validate_critical_env_vars();
        assert!(result.is_err(), "Should fail when JWT_SECRET is missing");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("JWT_SECRET"), "Error should mention JWT_SECRET");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_missing_smtp_server() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();
        
        // Set all except SMTP_SERVER
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/test");
        env::set_var("JWT_SECRET", "test_jwt_secret");
        env::set_var("ADMIN_EMAIL", "admin@test.com");

        let result = validate_critical_env_vars();
        assert!(result.is_err(), "Should fail when SMTP_SERVER is missing");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("SMTP_SERVER"), "Error should mention SMTP_SERVER");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_missing_admin_email() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();
        
        // Set all except ADMIN_EMAIL
        env::set_var("DATABASE_URL", "postgresql://test:test@localhost:5432/test");
        env::set_var("JWT_SECRET", "test_jwt_secret");
        env::set_var("SMTP_SERVER", "smtp.test.com");

        let result = validate_critical_env_vars();
        assert!(result.is_err(), "Should fail when ADMIN_EMAIL is missing");
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("ADMIN_EMAIL"), "Error should mention ADMIN_EMAIL");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_all_missing() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();

        let result = validate_critical_env_vars();
        assert!(result.is_err(), "Should fail when all env vars are missing");
        
        // Should fail on the first missing variable (DATABASE_URL)
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("DATABASE_URL"), "Error should mention DATABASE_URL");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_empty_values() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();
        
        // Set env vars to empty strings
        env::set_var("DATABASE_URL", "");
        env::set_var("JWT_SECRET", "");
        env::set_var("SMTP_SERVER", "");
        env::set_var("ADMIN_EMAIL", "");

        // Empty string values should still be considered "set" by env::var()
        let result = validate_critical_env_vars();
        assert!(result.is_ok(), "Should succeed even with empty string values");

        cleanup_test_env_vars();
    }

    #[test]
    fn test_validate_critical_env_vars_error_message_format() {
        let _guard = ENV_MUTEX.lock().unwrap();
        cleanup_test_env_vars();

        let result = validate_critical_env_vars();
        assert!(result.is_err());
        
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.starts_with("Missing required environment variable:"));
        assert!(error_message.contains("DATABASE_URL"));

        cleanup_test_env_vars();
    }

    // Note: setup_database function requires actual database connection and AppConfig
    // These would be better tested in integration tests rather than unit tests
    // Adding a simple test to verify the function signature and basic error handling
    
    #[test]
    fn test_setup_database_function_exists() {
        // This test just verifies the function exists and can be called
        // Actual database testing should be done in integration tests
        
        // We can't easily test setup_database without a real database connection
        // But we can verify the function signature compiles
        let _function_ref = setup_database;
        assert!(true, "setup_database function exists and compiles");
    }
}
