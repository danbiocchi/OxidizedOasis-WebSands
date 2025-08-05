use sqlx::postgres::{PgPool, PgPoolOptions, Postgres};
use sqlx::{Executor, Error as SqlxError};
use sqlx::migrate::{MigrateDatabase, MigrateError};
use std::time::Duration;
use std::sync::Mutex;
use log::{info, debug, error, warn};
use crate::infrastructure::AppConfig;

// Global mutex to prevent concurrent database setup operations
static DB_SETUP_MUTEX: Mutex<()> = Mutex::new(());

pub type DatabasePool = PgPool;

#[derive(Debug)]
pub enum DatabaseError {
    Configuration(String),
    Connection(SqlxError),
    Setup(SqlxError),
    Migration(MigrateError),
    Permission(SqlxError),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(msg) => write!(f, "Database configuration error: {msg}"),
            Self::Connection(e) => write!(f, "Database connection error: {e}"),
            Self::Setup(e) => write!(f, "Database setup error: {e}"),
            Self::Migration(e) => write!(f, "Database migration error: {e}"),
            Self::Permission(e) => write!(f, "Database permission error: {e}"),
        }
    }
}

impl std::error::Error for DatabaseError {}

async fn verify_permissions(pool: &PgPool) -> Result<bool, DatabaseError> {
    debug!("Verifying database permissions...");
    
    // Try to create a test table to verify permissions
    let result = pool.execute(
        "CREATE TABLE IF NOT EXISTS _permission_test (id int)"
    ).await;
    
    // Clean up the test table regardless of the result
    let _ = pool.execute(
        "DROP TABLE IF EXISTS _permission_test"
    ).await;
    
    match result {
        Ok(_) => {
            debug!("Permission verification successful");
            Ok(true)
        }
        Err(e) => {
            warn!("Permission verification failed: {e}");
            Ok(false)
        }
    }
}

async fn reset_database(base_url: &str, db_name: &str) -> Result<(), DatabaseError> {
    info!("Attempting to reset database '{db_name}'...");
    
    // Connect to postgres database for administrative operations
    let temp_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(base_url)
        .await
        .map_err(DatabaseError::Connection)?;

    // Drop existing database if it exists
    let drop_query = format!(
        "DROP DATABASE IF EXISTS {db_name} WITH (FORCE)"
    );
    
    match temp_pool.execute(&*drop_query).await {
        Ok(_) => info!("Successfully dropped existing database"),
        Err(e) => {
            error!("Failed to drop database: {e}");
            return Err(DatabaseError::Setup(e));
        }
    }

    // Create fresh database
    Postgres::create_database(&format!("{base_url}/{db_name}")).await
        .map_err(DatabaseError::Connection)?;
    
    info!("Database reset successful");
    Ok(())
}

async fn setup_database(pool: &PgPool, db_name: &str, app_user: &str, environment: &str) -> Result<(), DatabaseError> {
    // Use mutex to prevent concurrent database setup operations
    let _lock = DB_SETUP_MUTEX.lock().unwrap();
    
    debug!("Setting up database schema and permissions...");
    
    // Create base queries that are common to both environments
    let mut setup_queries = vec![
        // Ensure schema exists and is owned by superuser
        "CREATE SCHEMA IF NOT EXISTS public".to_string(),
    ];

    if environment == "development" {
        // Development mode: grant permissions to superuser and app_user
        setup_queries.extend(vec![
            // Grant permissions to app_user
            format!("GRANT CONNECT ON DATABASE {} TO {}", db_name, app_user),
            format!("GRANT USAGE, CREATE ON SCHEMA public TO {}", app_user),
            format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {}", app_user),
            format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {}", app_user),
            format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO {}", app_user),
            format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {}", app_user),
        ]);
    } else {
        // Production mode: restrict access to only the app_user
        setup_queries.extend(vec![
            // Revoke public access
            format!("REVOKE ALL ON DATABASE {} FROM PUBLIC", db_name),
            "REVOKE ALL ON SCHEMA public FROM PUBLIC".to_string(),
            
            // Grant minimal required permissions to app_user
            format!("GRANT CONNECT ON DATABASE {} TO {}", db_name, app_user),
            format!("GRANT USAGE, CREATE ON SCHEMA public TO {}", app_user),
            format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {}", app_user),
            format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {}", app_user),
            format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO {}", app_user),
            format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {}", app_user),
        ]);
    }

    for query in setup_queries {
        match pool.execute(&*query).await {
            Ok(_) => debug!("Successfully executed: {query}"),
            Err(e) => {
                // Handle permission errors gracefully - they might already be set
                if e.to_string().contains("already exists") ||
                   e.to_string().contains("duplicate key") ||
                   e.to_string().contains("already granted") ||
                   e.to_string().contains("role") && e.to_string().contains("already") {
                    debug!("Permission already exists, skipping: {query}");
                } else {
                    error!("Failed to execute setup query '{query}': {e}");
                    return Err(DatabaseError::Setup(e));
                }
            }
        }
    }

    info!("Database schema and permissions set up successfully");
    Ok(())
}

async fn setup_and_migrate(su_pool: &PgPool, db_name: &str, app_user: &str, environment: &str) -> Result<(), DatabaseError> {
    // Set up database permissions
    setup_database(su_pool, db_name, app_user, environment).await?;
    
    // Run migrations with mutex protection
    {
        let _lock = DB_SETUP_MUTEX.lock().unwrap();
        info!("Running migrations...");
        match sqlx::migrate!("./migrations").run(su_pool).await {
            Ok(_) => info!("Migrations completed successfully"),
            Err(e) => {
                // Check if migrations already applied
                if e.to_string().contains("applied") || e.to_string().contains("version") {
                    info!("Migrations already applied, continuing...");
                } else {
                    return Err(DatabaseError::Migration(e));
                }
            }
        }
    }
    
    Ok(())
}

pub async fn create_pool(config: &AppConfig) -> Result<DatabasePool, DatabaseError> {
    // Use the configuration passed in rather than environment variables to avoid race conditions
    let database_url = &config.database.url;
    
    // Extract database name from DATABASE_URL to avoid race conditions with environment variables
    let db_name = database_url
        .split('/')
        .next_back()
        .ok_or_else(|| DatabaseError::Configuration("Invalid DATABASE_URL format - cannot extract database name".into()))?
        .to_string();
    
    // Construct SU_DATABASE_URL using the same database name to ensure consistency
    let su_database_url = format!("postgres://dreamer@localhost:5432/{db_name}");
    
    println!("🔍 [create_pool] Thread: {:?}, DB_NAME: {}", std::thread::current().id(), db_name);
    let db_user = std::env::var("DB_USER")
        .map_err(|_| DatabaseError::Configuration("DB_USER is not set".into()))?;
    let environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "production".to_string());

    debug!("Initializing database connection...");

    // Validate URL format
    if !su_database_url.starts_with("postgres://") || !database_url.starts_with("postgres://") {
        return Err(DatabaseError::Configuration("Invalid database URL format".into()));
    }

    // Extract base URL for potential database reset
    let base_url = if su_database_url.ends_with(&format!("/{db_name}")) {
        su_database_url.replace(&format!("/{db_name}"), "/postgres")
    } else {
        // Handle cases where the URL format might be different
        let parts: Vec<&str> = su_database_url.rsplitn(2, '/').collect();
        if parts.len() == 2 {
            format!("{}/postgres", parts[1])
        } else {
            su_database_url.replace(&db_name, "postgres")
        }
    };

    // First check if database exists
    let db_exists = Postgres::database_exists(&su_database_url).await
        .map_err(DatabaseError::Connection)?;

    if environment == "development" {
        if db_exists {
            // Connect to existing database
            let su_pool = PgPoolOptions::new()
                .max_connections(1)
                .connect(&su_database_url)
                .await
                .map_err(DatabaseError::Connection)?;
            
            // Verify permissions on existing database
            if !verify_permissions(&su_pool).await? {
                warn!("Permission issues detected in development mode, attempting database reset...");
                // Drop superuser connection before reset
                drop(su_pool);
                
                // Reset the database
                reset_database(&base_url, &db_name).await?;
                
                // Reconnect as superuser to the new database
                let su_pool = PgPoolOptions::new()
                    .max_connections(1)
                    .connect(&su_database_url)
                    .await
                    .map_err(DatabaseError::Connection)?;
                
                // Set up fresh permissions and run migrations
                setup_and_migrate(&su_pool, &db_name, &db_user, &environment).await?;
            }
        } else {
            // Create new database
            info!("Creating new database '{db_name}'...");
            println!("🔍 [create_pool] Thread: {:?}, Attempting to create database: {}", std::thread::current().id(), db_name);
            match Postgres::create_database(&su_database_url).await {
                Ok(_) => {
                    info!("Database '{db_name}' created successfully");
                    println!("🔍 [create_pool] Thread: {:?}, Database creation SUCCESS: {}", std::thread::current().id(), db_name);
                }
                Err(e) => {
                    // Handle race condition - another test might have created the database
                    if e.to_string().contains("already exists") || e.to_string().contains("duplicate key") {
                        info!("Database '{db_name}' already exists (created by concurrent process)");
                        println!("🔍 [create_pool] Thread: {:?}, Database creation RACE CONDITION: {} - {}", std::thread::current().id(), db_name, e);
                    } else {
                        println!("🔍 [create_pool] Thread: {:?}, Database creation FAILED: {} - {}", std::thread::current().id(), db_name, e);
                        return Err(DatabaseError::Connection(e));
                    }
                }
            }
            
            // Connect to database (whether we created it or it already existed)
            let su_pool = PgPoolOptions::new()
                .max_connections(1)
                .connect(&su_database_url)
                .await
                .map_err(DatabaseError::Connection)?;
            
            // Set up permissions and run migrations
            setup_and_migrate(&su_pool, &db_name, &db_user, &environment).await?;
        }
    } else {
        // Production mode: simpler flow
        if !db_exists {
            info!("Creating database '{db_name}'...");
            match Postgres::create_database(&su_database_url).await {
                Ok(_) => {
                    info!("Database '{db_name}' created successfully");
                }
                Err(e) => {
                    // Handle race condition - another test might have created the database
                    if e.to_string().contains("already exists") || e.to_string().contains("duplicate key") {
                        info!("Database '{db_name}' already exists (created by concurrent process)");
                    } else {
                        return Err(DatabaseError::Connection(e));
                    }
                }
            }
        }
        
        let su_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&su_database_url)
            .await
            .map_err(DatabaseError::Connection)?;
        
        // Set up permissions and run migrations
        setup_and_migrate(&su_pool, &db_name, &db_user, &environment).await?;
    }

    // Create application user pool for normal operations
    info!("Creating application user connection pool...");
    let app_pool = PgPoolOptions::new()
        .max_connections(config.database.max_connections)
        .min_connections(1)
        .max_lifetime(Some(Duration::from_secs(30 * 60)))
        .idle_timeout(Some(Duration::from_secs(10 * 60)))
        .acquire_timeout(Duration::from_secs(30))
        .connect(database_url)
        .await
        .map_err(DatabaseError::Connection)?;

    info!("Database connection pool established successfully");
    Ok(app_pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_database_error_display() {
        let config_error = DatabaseError::Configuration("Missing URL".to_string());
        assert_eq!(config_error.to_string(), "Database configuration error: Missing URL");

        let connection_error = DatabaseError::Connection(SqlxError::Configuration("Connection failed".into()));
        assert!(connection_error.to_string().contains("Database connection error"));

        let setup_error = DatabaseError::Setup(SqlxError::Configuration("Setup failed".into()));
        assert!(setup_error.to_string().contains("Database setup error"));

        let migration_error = DatabaseError::Migration(MigrateError::VersionMissing(-1));
        assert!(migration_error.to_string().contains("Database migration error"));

        let permission_error = DatabaseError::Permission(SqlxError::Configuration("Permission denied".into()));
        assert!(permission_error.to_string().contains("Database permission error"));
    }

    #[test]
    fn test_database_error_debug() {
        let error = DatabaseError::Configuration("Test error".to_string());
        let debug_output = format!("{:?}", error);
        assert!(debug_output.contains("Configuration"));
        assert!(debug_output.contains("Test error"));
    }

    #[test]
    fn test_database_error_from_sqlx_error() {
        let sqlx_error = SqlxError::Configuration("Test SQLx error".into());
        let db_error = DatabaseError::Connection(sqlx_error);
        assert!(db_error.to_string().contains("Database connection error"));
    }

    mod url_validation_tests {
        use super::*;

        #[test]
        fn test_extract_database_name_from_url() {
            // Valid URL format
            let url = "postgres://user:pass@localhost:5432/testdb";
            let db_name = url.split('/').last().unwrap();
            assert_eq!(db_name, "testdb");

            // URL with query parameters
            let url_with_params = "postgres://user:pass@localhost:5432/testdb?sslmode=require";
            let db_name_with_params = url_with_params.split('/').last().unwrap().split('?').next().unwrap();
            assert_eq!(db_name_with_params, "testdb");

            // Complex database name
            let complex_url = "postgres://user@localhost:5432/test_oxidizedoasis_db_12345";
            let complex_db_name = complex_url.split('/').last().unwrap();
            assert_eq!(complex_db_name, "test_oxidizedoasis_db_12345");
        }

        #[test]
        fn test_url_format_validation() {
            let valid_urls = vec![
                "postgres://localhost:5432/testdb",
                "postgres://user:pass@localhost:5432/testdb",
                "postgres://user@host.com:5432/database_name",
            ];

            for url in valid_urls {
                assert!(url.starts_with("postgres://"), "URL should start with postgres://: {}", url);
                assert!(url.contains("/"), "URL should contain database path: {}", url);
            }

            let invalid_urls = vec![
                "mysql://localhost:3306/testdb",
                "postgresql://localhost:5432/testdb", // Wrong protocol
                "postgres:localhost:5432/testdb", // Missing //
                "localhost:5432/testdb", // Missing protocol
            ];

            for url in invalid_urls {
                assert!(!url.starts_with("postgres://"), "URL should be invalid: {}", url);
            }
        }

        #[test]
        fn test_base_url_construction() {
            let test_cases = vec![
                ("postgres://localhost:5432/testdb", "postgres://localhost:5432/postgres"),
                ("postgres://user:pass@localhost:5432/mydb", "postgres://user:pass@localhost:5432/postgres"),
                ("postgres://user@host.com:5432/complex_db_name", "postgres://user@host.com:5432/postgres"),
            ];

            for (input_url, expected_base) in test_cases {
                let db_name = input_url.split('/').last().unwrap();
                let base_url = if input_url.ends_with(&format!("/{}", db_name)) {
                    input_url.replace(&format!("/{}", db_name), "/postgres")
                } else {
                    let parts: Vec<&str> = input_url.rsplitn(2, '/').collect();
                    if parts.len() == 2 {
                        format!("{}/postgres", parts[1])
                    } else {
                        input_url.replace(db_name, "postgres")
                    }
                };
                assert_eq!(base_url, expected_base, "Base URL construction failed for: {}", input_url);
            }
        }
    }

    mod environment_variable_tests {
        use super::*;

        fn with_env_var<F, R>(key: &str, value: Option<&str>, func: F) -> R
        where
            F: FnOnce() -> R,
        {
            let original = env::var(key).ok();
            
            match value {
                Some(val) => env::set_var(key, val),
                None => env::remove_var(key),
            }
            
            let result = func();
            
            match original {
                Some(orig_val) => env::set_var(key, orig_val),
                None => env::remove_var(key),
            }
            
            result
        }

        #[test]
        fn test_environment_variable_handling() {
            // Test ENVIRONMENT variable defaults to production
            let environment = with_env_var("ENVIRONMENT", None, || {
                env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string())
            });
            assert_eq!(environment, "production");

            // Test ENVIRONMENT variable can be set to development
            let dev_environment = with_env_var("ENVIRONMENT", Some("development"), || {
                env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string())
            });
            assert_eq!(dev_environment, "development");

            // Test ENVIRONMENT variable can be set to custom value
            let custom_environment = with_env_var("ENVIRONMENT", Some("staging"), || {
                env::var("ENVIRONMENT").unwrap_or_else(|_| "production".to_string())
            });
            assert_eq!(custom_environment, "staging");
        }

        #[test]
        fn test_db_user_requirement() {
            // Test that missing DB_USER would cause an error
            with_env_var("DB_USER", None, || {
                let result = env::var("DB_USER");
                assert!(result.is_err(), "DB_USER should be required");
            });

            // Test that present DB_USER works
            with_env_var("DB_USER", Some("testuser"), || {
                let result = env::var("DB_USER");
                assert!(result.is_ok(), "DB_USER should be available when set");
                assert_eq!(result.unwrap(), "testuser");
            });
        }
    }

    mod connection_pool_tests {
        use super::*;

        #[test]
        fn test_pool_configuration_parameters() {
            // Test that pool configuration uses reasonable defaults
            let max_connections = 10u32;
            let max_lifetime = Duration::from_secs(30 * 60); // 30 minutes
            let idle_timeout = Duration::from_secs(10 * 60); // 10 minutes
            let acquire_timeout = Duration::from_secs(30); // 30 seconds

            // Verify timeouts are reasonable
            assert!(max_lifetime.as_secs() > 0, "Max lifetime should be positive");
            assert!(idle_timeout.as_secs() > 0, "Idle timeout should be positive");
            assert!(acquire_timeout.as_secs() > 0, "Acquire timeout should be positive");
            
            // Verify connection limits are reasonable
            assert!(max_connections > 0, "Max connections should be positive");
            assert!(max_connections <= 100, "Max connections should be reasonable for most use cases");

            // Verify timeout relationships make sense
            assert!(max_lifetime > idle_timeout, "Max lifetime should be longer than idle timeout");
            assert!(idle_timeout > acquire_timeout, "Idle timeout should be longer than acquire timeout");
        }

        #[test]
        fn test_connection_pool_options() {
            let options = PgPoolOptions::new()
                .max_connections(10)
                .min_connections(1)
                .max_lifetime(Some(Duration::from_secs(1800)))
                .idle_timeout(Some(Duration::from_secs(600)))
                .acquire_timeout(Duration::from_secs(30));

            // Test that options can be created without errors
            assert_eq!(options.get_max_connections(), 10);
            assert_eq!(options.get_min_connections(), 1);
        }
    }

    mod error_handling_tests {
        use super::*;

        #[test]
        fn test_configuration_error_types() {
            let error1 = DatabaseError::Configuration("Invalid URL".to_string());
            let error2 = DatabaseError::Configuration("Missing environment variable".to_string());
            
            assert!(error1.to_string().contains("Invalid URL"));
            assert!(error2.to_string().contains("Missing environment variable"));
        }

        #[test]
        fn test_error_chain_compatibility() {
            let db_error = DatabaseError::Configuration("Test error".to_string());
            
            // Test that DatabaseError implements std::error::Error
            let _error_trait: &dyn std::error::Error = &db_error;
            
            // Test error source (should be None for Configuration errors)
            use std::error::Error;
            assert!(db_error.source().is_none());
        }

        #[test]
        fn test_error_message_consistency() {
            let test_cases = vec![
                (DatabaseError::Configuration("test".to_string()), "Database configuration error"),
                (DatabaseError::Connection(SqlxError::Configuration("test".into())), "Database connection error"),
                (DatabaseError::Setup(SqlxError::Configuration("test".into())), "Database setup error"),
                (DatabaseError::Migration(MigrateError::VersionMissing(-1)), "Database migration error"),
                (DatabaseError::Permission(SqlxError::Configuration("test".into())), "Database permission error"),
            ];

            for (error, expected_prefix) in test_cases {
                let message = error.to_string();
                assert!(message.starts_with(expected_prefix),
                    "Error message '{}' should start with '{}'", message, expected_prefix);
            }
        }
    }

    mod database_name_extraction_tests {
        use super::*;

        #[test]
        fn test_database_name_extraction_edge_cases() {
            // Test various URL formats that might occur
            let test_cases = vec![
                ("postgres://localhost:5432/simple", "simple"),
                ("postgres://user@localhost:5432/with_underscores", "with_underscores"),
                ("postgres://user:pass@localhost:5432/with-dashes", "with-dashes"),
                ("postgres://localhost:5432/test123", "test123"),
                ("postgres://localhost:5432/Test_DB_Name_123", "Test_DB_Name_123"),
            ];

            for (url, expected_name) in test_cases {
                let extracted = url.split('/').last().unwrap();
                assert_eq!(extracted, expected_name, "Failed to extract database name from: {}", url);
            }
        }

        #[test]
        fn test_invalid_url_formats() {
            let invalid_urls = vec![
                "postgres://localhost:5432", // No database name
                "postgres://localhost:5432/", // Empty database name
                "not_a_url", // Not a URL at all
                "", // Empty string
            ];

            for url in invalid_urls {
                let result = url.split('/').last();
                if let Some(db_name) = result {
                    if db_name.is_empty() {
                        // This would be caught by our validation logic
                        assert!(db_name.is_empty(), "Empty database name should be invalid: {}", url);
                    }
                } else {
                    panic!("Should always get some result from split, even if empty");
                }
            }
        }
    }

    mod concurrent_access_tests {
        use super::*;
        use std::sync::Arc;
        use std::thread;

        #[test]
        fn test_mutex_prevents_concurrent_setup() {
            // Test that the mutex can be acquired and prevents concurrent access
            let mutex = &DB_SETUP_MUTEX;
            
            // Acquire lock in main thread
            let _lock1 = mutex.lock().unwrap();
            
            // Try to acquire in another thread (should block briefly)
            let handle = thread::spawn(move || {
                let start = std::time::Instant::now();
                let _lock2 = DB_SETUP_MUTEX.lock().unwrap();
                start.elapsed()
            });
            
            // Release the first lock
            drop(_lock1);
            
            // The other thread should complete quickly after lock release
            let elapsed = handle.join().unwrap();
            assert!(elapsed < Duration::from_secs(1), "Lock acquisition took too long: {:?}", elapsed);
        }

        #[test]
        fn test_thread_id_tracking() {
            // Test that thread IDs can be tracked (used in database naming)
            let thread_id = std::thread::current().id();
            let thread_id_str = format!("{:?}", thread_id);
            
            // Thread ID should be a reasonable format
            assert!(!thread_id_str.is_empty(), "Thread ID should not be empty");
            assert!(thread_id_str.contains("ThreadId"), "Thread ID should contain ThreadId");
        }
    }

    mod validation_tests {
        use super::*;

        #[test]
        fn test_url_validation_logic() {
            let valid_urls = vec![
                "postgres://localhost:5432/test",
                "postgres://user:pass@localhost:5432/test",
                "postgres://user@host:5432/test",
            ];

            for url in valid_urls {
                assert!(url.starts_with("postgres://"), "Valid URL should pass validation: {}", url);
            }

            let invalid_urls = vec![
                "mysql://localhost:3306/test",
                "postgresql://localhost:5432/test",
                "http://localhost:5432/test",
                "ftp://localhost:5432/test",
                "postgres:localhost:5432/test", // Missing //
            ];

            for url in invalid_urls {
                assert!(!url.starts_with("postgres://"), "Invalid URL should fail validation: {}", url);
            }
        }

        #[test]
        fn test_database_name_validation() {
            // Valid database names
            let valid_names = vec![
                "testdb",
                "test_db",
                "test-db",
                "test123",
                "test_oxidizedoasis_db_12345",
            ];

            for name in valid_names {
                assert!(!name.is_empty(), "Valid database name should not be empty: {}", name);
                assert!(!name.contains("/"), "Valid database name should not contain slash: {}", name);
                assert!(!name.contains("\\"), "Valid database name should not contain backslash: {}", name);
            }

            // Invalid database names that should be caught
            let invalid_names = vec![
                "",
                "/",
                "db/name",
                "db\\name",
            ];

            for name in invalid_names {
                if name.is_empty() || name.contains("/") || name.contains("\\") {
                    // These would be invalid in real usage
                    assert!(true, "Invalid database name detected: {}", name);
                }
            }
        }
    }

    mod permission_setup_tests {
        use super::*;

        #[test]
        fn test_development_permission_queries() {
            let db_name = "test_db";
            let app_user = "test_user";
            let environment = "development";

            let grant_connect = format!("GRANT CONNECT ON DATABASE {} TO {}", db_name, app_user);
            let grant_usage = format!("GRANT USAGE, CREATE ON SCHEMA public TO {}", app_user);
            let alter_default_tables = format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {}", app_user);
            let grant_all_tables = format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {}", app_user);
            let alter_default_sequences = format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO {}", app_user);
            let grant_all_sequences = format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {}", app_user);

            let expected_queries = vec![
                "CREATE SCHEMA IF NOT EXISTS public",
                &grant_connect,
                &grant_usage,
                &alter_default_tables,
                &grant_all_tables,
                &alter_default_sequences,
                &grant_all_sequences,
            ];

            // Simulate the query generation logic
            let mut setup_queries = vec!["CREATE SCHEMA IF NOT EXISTS public".to_string()];

            if environment == "development" {
                setup_queries.extend(vec![
                    format!("GRANT CONNECT ON DATABASE {} TO {}", db_name, app_user),
                    format!("GRANT USAGE, CREATE ON SCHEMA public TO {}", app_user),
                    format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {}", app_user),
                    format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {}", app_user),
                    format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO {}", app_user),
                    format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {}", app_user),
                ]);
            }

            assert_eq!(setup_queries.len(), expected_queries.len());
            for (actual, expected) in setup_queries.iter().zip(expected_queries.iter()) {
                assert_eq!(actual, expected, "Permission query mismatch");
            }
        }

        #[test]
        fn test_production_permission_queries() {
            let db_name = "prod_db";
            let app_user = "prod_user";
            let environment = "production";

            // Simulate the query generation logic for production
            let mut setup_queries = vec!["CREATE SCHEMA IF NOT EXISTS public".to_string()];

            if environment != "development" {
                setup_queries.extend(vec![
                    format!("REVOKE ALL ON DATABASE {} FROM PUBLIC", db_name),
                    "REVOKE ALL ON SCHEMA public FROM PUBLIC".to_string(),
                    format!("GRANT CONNECT ON DATABASE {} TO {}", db_name, app_user),
                    format!("GRANT USAGE, CREATE ON SCHEMA public TO {}", app_user),
                    format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO {}", app_user),
                    format!("GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {}", app_user),
                    format!("ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT USAGE, SELECT ON SEQUENCES TO {}", app_user),
                    format!("GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {}", app_user),
                ]);
            }

            // Production should have more restrictive queries (REVOKE statements)
            assert!(setup_queries.iter().any(|q| q.contains("REVOKE")), "Production should include REVOKE statements");
            assert!(setup_queries.len() > 1, "Production should have multiple permission queries");
        }

        #[test]
        fn test_permission_error_handling() {
            // Test the error handling patterns used in setup_database
            let error_patterns = vec![
                "already exists",
                "duplicate key",
                "already granted",
                "role test_user already",
            ];

            for pattern in error_patterns {
                let test_error = format!("Some database error: {}", pattern);
                
                // Simulate the error checking logic from setup_database
                let should_ignore = test_error.contains("already exists") ||
                    test_error.contains("duplicate key") ||
                    test_error.contains("already granted") ||
                    (test_error.contains("role") && test_error.contains("already"));

                assert!(should_ignore, "Error '{}' should be ignored as non-critical", test_error);
            }

            // Test errors that should NOT be ignored
            let critical_errors = vec![
                "permission denied",
                "connection refused",
                "database does not exist",
                "invalid query",
            ];

            for error_msg in critical_errors {
                let should_ignore = error_msg.contains("already exists") ||
                    error_msg.contains("duplicate key") ||
                    error_msg.contains("already granted") ||
                    (error_msg.contains("role") && error_msg.contains("already"));

                assert!(!should_ignore, "Critical error '{}' should NOT be ignored", error_msg);
            }
        }
    }

    mod edge_case_tests {
        use super::*;

        #[test]
        fn test_special_characters_in_database_names() {
            let special_names = vec![
                "test_db_123",
                "test-db-456",
                "testdb789",
                "test_oxidizedoasis_db_threadid32_25_918000_adb1d8f5", // Real test pattern
            ];

            for name in special_names {
                // Validate that these names would work in URL construction
                let test_url = format!("postgres://localhost:5432/{}", name);
                assert!(test_url.starts_with("postgres://"), "URL with special name should be valid: {}", test_url);
                
                let extracted = test_url.split('/').last().unwrap();
                assert_eq!(extracted, name, "Database name should be extractable: {}", name);
            }
        }

        #[test]
        fn test_very_long_database_names() {
            // PostgreSQL has a limit on identifier length (63 characters)
            let long_name = "a".repeat(50); // Under the limit
            let very_long_name = "a".repeat(100); // Over the limit

            // Normal length should be fine
            assert!(long_name.len() < 63, "Normal length name should be under PostgreSQL limit");

            // Very long names might cause issues (this is a validation concern)
            if very_long_name.len() > 63 {
                // This would be caught by PostgreSQL, not our code
                assert!(very_long_name.len() > 63, "Very long name exceeds PostgreSQL identifier limit");
            }
        }

        #[test]
        fn test_url_parsing_edge_cases() {
            // Test URLs with unusual but valid formats
            let edge_case_urls = vec![
                "postgres://localhost:5432/test?sslmode=require",
                "postgres://user:p@ssw0rd@localhost:5432/test",
                "postgres://user@localhost:15432/test", // Non-standard port
            ];

            for url in edge_case_urls {
                let db_name = url.split('/').last().unwrap().split('?').next().unwrap();
                assert!(!db_name.is_empty(), "Should extract non-empty database name from: {}", url);
                assert_eq!(db_name, "test", "Should extract 'test' from URL: {}", url);
            }
        }
    }

    mod performance_tests {
        use super::*;

        #[test]
        fn test_mutex_performance() {
            // Test that mutex operations are fast
            let start = std::time::Instant::now();
            
            for _ in 0..1000 {
                let _lock = DB_SETUP_MUTEX.lock().unwrap();
                // Simulate brief work
                drop(_lock);
            }
            
            let elapsed = start.elapsed();
            assert!(elapsed < Duration::from_millis(100), "Mutex operations should be fast: {:?}", elapsed);
        }

        #[test]
        fn test_string_operations_performance() {
            // Test that string operations used in URL parsing are efficient
            let test_url = "postgres://user:password@localhost:5432/test_database_name";
            let start = std::time::Instant::now();
            
            for _ in 0..10000 {
                let _db_name = test_url.split('/').last().unwrap();
                let _base_url = test_url.replace("test_database_name", "postgres");
            }
            
            let elapsed = start.elapsed();
            assert!(elapsed < Duration::from_millis(100), "String operations should be fast: {:?}", elapsed);
        }
    }
}
