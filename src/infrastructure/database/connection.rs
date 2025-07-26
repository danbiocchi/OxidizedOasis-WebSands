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
            Self::Configuration(msg) => write!(f, "Database configuration error: {}", msg),
            Self::Connection(e) => write!(f, "Database connection error: {}", e),
            Self::Setup(e) => write!(f, "Database setup error: {}", e),
            Self::Migration(e) => write!(f, "Database migration error: {}", e),
            Self::Permission(e) => write!(f, "Database permission error: {}", e),
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
            warn!("Permission verification failed: {}", e);
            Ok(false)
        }
    }
}

async fn reset_database(base_url: &str, db_name: &str) -> Result<(), DatabaseError> {
    info!("Attempting to reset database '{}'...", db_name);
    
    // Connect to postgres database for administrative operations
    let temp_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(base_url)
        .await
        .map_err(DatabaseError::Connection)?;

    // Drop existing database if it exists
    let drop_query = format!(
        "DROP DATABASE IF EXISTS {} WITH (FORCE)",
        db_name
    );
    
    match temp_pool.execute(&*drop_query).await {
        Ok(_) => info!("Successfully dropped existing database"),
        Err(e) => {
            error!("Failed to drop database: {}", e);
            return Err(DatabaseError::Setup(e));
        }
    }

    // Create fresh database
    Postgres::create_database(&format!("{}/{}", base_url, db_name)).await
        .map_err(|e| DatabaseError::Connection(e))?;
    
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
            Ok(_) => debug!("Successfully executed: {}", query),
            Err(e) => {
                // Handle permission errors gracefully - they might already be set
                if e.to_string().contains("already exists") ||
                   e.to_string().contains("duplicate key") ||
                   e.to_string().contains("already granted") ||
                   e.to_string().contains("role") && e.to_string().contains("already") {
                    debug!("Permission already exists, skipping: {}", query);
                } else {
                    error!("Failed to execute setup query '{}': {}", query, e);
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
        .last()
        .ok_or_else(|| DatabaseError::Configuration("Invalid DATABASE_URL format - cannot extract database name".into()))?
        .to_string();
    
    // Construct SU_DATABASE_URL using the same database name to ensure consistency
    let su_database_url = format!("postgres://dreamer@localhost:5432/{}", db_name);
    
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
    let base_url = if su_database_url.ends_with(&format!("/{}", db_name)) {
        su_database_url.replace(&format!("/{}", db_name), "/postgres")
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
            info!("Creating new database '{}'...", db_name);
            println!("🔍 [create_pool] Thread: {:?}, Attempting to create database: {}", std::thread::current().id(), db_name);
            match Postgres::create_database(&su_database_url).await {
                Ok(_) => {
                    info!("Database '{}' created successfully", db_name);
                    println!("🔍 [create_pool] Thread: {:?}, Database creation SUCCESS: {}", std::thread::current().id(), db_name);
                }
                Err(e) => {
                    // Handle race condition - another test might have created the database
                    if e.to_string().contains("already exists") || e.to_string().contains("duplicate key") {
                        info!("Database '{}' already exists (created by concurrent process)", db_name);
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
            info!("Creating database '{}'...", db_name);
            match Postgres::create_database(&su_database_url).await {
                Ok(_) => {
                    info!("Database '{}' created successfully", db_name);
                }
                Err(e) => {
                    // Handle race condition - another test might have created the database
                    if e.to_string().contains("already exists") || e.to_string().contains("duplicate key") {
                        info!("Database '{}' already exists (created by concurrent process)", db_name);
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
        .max_connections(config.database.max_connections as u32)
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
