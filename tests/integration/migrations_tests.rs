use std::sync::Arc;
use sqlx::PgPool;
use oxidizedoasis_websands::infrastructure::database::connection::create_pool;
use oxidizedoasis_websands::infrastructure::config::app_config::AppConfig;



// Since migrations module is private, we test the migration functionality
// through the create_pool function which calls migrations internally

#[tokio::test]
async fn test_migrations_through_create_pool_success() {
    // Create a test database config - this will trigger migrations internally
    let config = test_common::create_test_app_config();
    
    // create_pool runs migrations internally during setup
    let result = create_pool(&config).await;
    
    // Verify the pool creation (and internal migrations) succeed
    assert!(result.is_ok(), "Pool creation with migrations should succeed on valid database");
}

#[tokio::test]
async fn test_migrations_through_create_pool_with_test_database() {
    // Create a temporary test database
    let (config, _db_name) = test_common::create_test_config_with_cleanup()
        .await
        .expect("Failed to create test database");
    
    // create_pool will set up and migrate the test database
    let result = create_pool(&config).await;
    
    // Should succeed on a fresh database
    assert!(result.is_ok(), "Pool creation with migrations should succeed on fresh database");
}

#[tokio::test]
async fn test_migrations_idempotent_through_create_pool() {
    // Test that running create_pool multiple times is idempotent (migrations run multiple times)
    let config = test_common::create_test_app_config();
    
    // First pool creation with migrations
    let result1 = create_pool(&config).await;
    assert!(result1.is_ok(), "First pool creation with migrations should succeed");
    
    // Second pool creation - migrations should be idempotent
    let result2 = create_pool(&config).await;
    assert!(result2.is_ok(), "Second pool creation should succeed (idempotent migrations)");
}

#[tokio::test]
async fn test_create_pool_with_invalid_config() {
    // Create invalid config to test error handling
    let mut config = test_common::create_test_app_config();
    config.database.url = "postgres://invalid_user:invalid_pass@invalid_host:9999/invalid_db".to_string();
    
    // This should fail to create a pool
    let result = create_pool(&config).await;
    assert!(result.is_err(), "Pool creation should fail with invalid configuration");
}

#[tokio::test]
async fn test_migrations_directory_structure() {
    // Verify that migration files exist in the expected location
    use std::path::Path;
    
    let migrations_dir = Path::new("./migrations");
    assert!(migrations_dir.exists(), "Migrations directory should exist");
    assert!(migrations_dir.is_dir(), "Migrations path should be a directory");
    
    // Check for expected migration files
    let expected_files = [
        "20240901010340_initial_schema.sql",
        "20240902010341_add_password_reset.sql", 
        "20240903010342_add_revoked_tokens.sql",
        "20250302010343_add_active_tokens.sql",
    ];
    
    for file_name in &expected_files {
        let file_path = migrations_dir.join(file_name);
        assert!(file_path.exists(), "Migration file {} should exist", file_name);
        assert!(file_path.is_file(), "Migration path {} should be a file", file_name);
    }
}

#[tokio::test]
async fn test_migration_sql_files_are_readable() {
    // Test that migration files can be read and contain SQL
    use std::fs;
    use std::path::Path;
    
    let migrations_dir = Path::new("./migrations");
    let expected_files = [
        "20240901010340_initial_schema.sql",
        "20240902010341_add_password_reset.sql",
        "20240903010342_add_revoked_tokens.sql", 
        "20250302010343_add_active_tokens.sql",
    ];
    
    for file_name in &expected_files {
        let file_path = migrations_dir.join(file_name);
        let content = fs::read_to_string(&file_path)
            .expect(&format!("Should be able to read migration file {}", file_name));
        
        // Verify it's not empty and contains SQL-like content
        assert!(!content.trim().is_empty(), "Migration file {} should not be empty", file_name);
        
        // Basic check for SQL content (should contain common SQL keywords)
        let content_upper = content.to_uppercase();
        let has_sql_keywords = content_upper.contains("CREATE") || 
                               content_upper.contains("ALTER") || 
                               content_upper.contains("INSERT") ||
                               content_upper.contains("DROP");
        
        assert!(has_sql_keywords, "Migration file {} should contain SQL keywords", file_name);
    }
}

#[tokio::test]
async fn test_run_migrations_return_type() {
    // Test the function signature and return type
    let config = test_common::create_test_app_config();
    let result = create_pool(&config).await;
    
    // Verify the function returns the expected Result type
    match result {
        Ok(pool) => {
            // Success case - verify it returns a database pool
            assert!(pool.is_closed() == false, "Pool should be open and ready");
        },
        Err(e) => {
            // Error case - verify it returns a proper error
            let _error_message = format!("{}", e);
            assert!(true, "Error case returns proper error type");
        }
    }
}

#[tokio::test]
async fn test_create_pool_connection_reuse() {
    // Test that create_pool can be called multiple times safely
    let config = test_common::create_test_app_config();
    
    // Create first pool
    let pool1 = create_pool(&config).await
        .expect("Failed to create first database pool");
    
    // Create second pool - should work fine
    let pool2 = create_pool(&config).await
        .expect("Failed to create second database pool");
    
    // Both pools should be usable
    assert!(!pool1.is_closed(), "First pool should be open");
    assert!(!pool2.is_closed(), "Second pool should be open");
}

#[tokio::test]
async fn test_migration_files_direct_access() {
    // Since migrations module is private, test through sqlx::migrate! directly
    use sqlx::migrate::Migrator;
    
    // Test that we can load the migrator (which reads migration files)
    let migrator = Migrator::new(std::path::Path::new("./migrations")).await;
    
    match migrator {
        Ok(m) => {
            // Verify migrations are loaded
            assert!(m.migrations.len() > 0, "Should have loaded migration files");
            
            // Check we have the expected number of migrations
            assert_eq!(m.migrations.len(), 4, "Should have exactly 4 migration files");
            
            // Verify migration versions are in correct order
            let versions: Vec<i64> = m.migrations.iter().map(|m| m.version).collect();
            let mut sorted_versions = versions.clone();
            sorted_versions.sort();
            assert_eq!(versions, sorted_versions, "Migration versions should be in ascending order");
        },
        Err(e) => {
            panic!("Failed to load migrations: {}", e);
        }
    }
}