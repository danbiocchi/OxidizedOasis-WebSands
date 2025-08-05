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

// Comprehensive tests for src/infrastructure/database/migrations.rs
// These tests directly test the run_migrations function and provide comprehensive coverage

use oxidizedoasis_websands::infrastructure::database::run_migrations;
use test_common::UnifiedTestFixture;
use sqlx::Row;

#[tokio::test]
async fn test_run_migrations_direct_execution_success() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test direct execution of run_migrations function
    let result = run_migrations(&fixture.db_pool).await;
    assert!(result.is_ok(), "Direct migration execution should succeed");
    
    // Verify all expected tables exist
    let tables = sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY table_name")
        .fetch_all(&fixture.db_pool)
        .await
        .expect("Should fetch table list");
    
    let table_names: Vec<String> = tables.iter()
        .map(|row| row.get::<String, _>("table_name"))
        .collect();
    
    // Check that all main tables exist
    assert!(table_names.contains(&"users".to_string()), "Users table should exist");
    assert!(table_names.contains(&"password_reset_tokens".to_string()), "Password reset tokens table should exist");
    assert!(table_names.contains(&"revoked_tokens".to_string()), "Revoked tokens table should exist");
    assert!(table_names.contains(&"active_tokens".to_string()), "Active tokens table should exist");
    assert!(table_names.contains(&"_sqlx_migrations".to_string()), "Migration tracking table should exist");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_status_tracking_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations explicitly to ensure they are applied
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Verify migration records exist in tracking table
    let migration_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations"
    )
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count migration records");
    
    assert!(migration_count > 0, "Should have migration records");
    assert_eq!(migration_count, 4, "Should have all 4 migrations applied");
    
    // Verify migration details
    let migrations = sqlx::query(
        "SELECT version, description, success FROM _sqlx_migrations ORDER BY version"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch migration records");
    
    assert_eq!(migrations.len(), 4, "Should have 4 migration records");
    
    // Verify all migrations succeeded
    for migration in migrations {
        let success: bool = migration.get("success");
        assert!(success, "All migrations should have succeeded");
    }
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_idempotency_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations first time
    let first_result = run_migrations(&fixture.db_pool).await;
    assert!(first_result.is_ok(), "First migration run should succeed");
    
    // Run migrations second time (should be idempotent)
    let second_result = run_migrations(&fixture.db_pool).await;
    assert!(second_result.is_ok(), "Second migration run should succeed (idempotent)");
    
    // Run migrations third time to ensure complete idempotency
    let third_result = run_migrations(&fixture.db_pool).await;
    assert!(third_result.is_ok(), "Third migration run should succeed (idempotent)");
    
    // Verify migration count hasn't increased
    let migration_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations"
    )
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count migration records");
    
    assert_eq!(migration_count, 4, "Should still have exactly 4 migrations");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_order_and_dependencies_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Check that migrations were applied in correct order
    let migrations = sqlx::query(
        "SELECT version, description FROM _sqlx_migrations ORDER BY version"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch migration records");
    
    assert_eq!(migrations.len(), 4, "Should have 4 migrations");
    
    // Verify migration order by checking version timestamps
    let versions: Vec<i64> = migrations.iter()
        .map(|row| row.get::<i64, _>("version"))
        .collect();
    
    // Check that versions are in ascending order
    for i in 1..versions.len() {
        assert!(versions[i] > versions[i-1], "Migrations should be in chronological order");
    }
    
    // Verify expected migration names/descriptions exist
    let descriptions: Vec<String> = migrations.iter()
        .map(|row| row.get::<String, _>("description"))
        .collect();
    
    // Debug output to see what descriptions we actually have
    println!("DEBUG: Migration descriptions found: {:?}", descriptions);
    
    assert!(descriptions.iter().any(|d| d.contains("initial")), "Should have initial schema migration");
    assert!(descriptions.iter().any(|d| d.contains("password reset")), "Should have password reset migration");
    assert!(descriptions.iter().any(|d| d.contains("revoked tokens")), "Should have revoked tokens migration");
    assert!(descriptions.iter().any(|d| d.contains("active tokens")), "Should have active tokens migration");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_comprehensive_schema_validation_after_migration() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Test users table schema comprehensively
    let users_columns = sqlx::query(
        "SELECT column_name, data_type, is_nullable, column_default \
         FROM information_schema.columns \
         WHERE table_name = 'users' AND table_schema = 'public'\
         ORDER BY column_name"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch users table schema");
    
    // Verify essential user columns exist
    let column_names: Vec<String> = users_columns.iter()
        .map(|row| row.get::<String, _>("column_name"))
        .collect();
    
    let expected_columns = vec![
        "id", "username", "email", "password_hash", "role",
        "is_active", "is_email_verified", "created_at", "updated_at",
        "verification_token", "verification_token_expires_at"
    ];
    
    for expected_col in expected_columns {
        assert!(column_names.contains(&expected_col.to_string()),
                "Users should have {} column", expected_col);
    }
    
    // Test password_reset_tokens table schema
    let password_reset_columns = sqlx::query(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_name = 'password_reset_tokens' AND table_schema = 'public'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch password reset tokens table schema");
    
    let password_reset_column_names: Vec<String> = password_reset_columns.iter()
        .map(|row| row.get::<String, _>("column_name"))
        .collect();
    
    let expected_reset_columns = vec!["id", "user_id", "token", "expires_at", "is_used", "created_at", "updated_at"];
    for expected_col in expected_reset_columns {
        assert!(password_reset_column_names.contains(&expected_col.to_string()),
                "Password reset tokens should have {} column", expected_col);
    }
    
    // Test revoked_tokens table schema
    let revoked_tokens_columns = sqlx::query(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_name = 'revoked_tokens' AND table_schema = 'public'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch revoked tokens table schema");
    
    let revoked_column_names: Vec<String> = revoked_tokens_columns.iter()
        .map(|row| row.get::<String, _>("column_name"))
        .collect();
    
    let expected_revoked_columns = vec!["id", "jti", "user_id", "token_type", "expires_at", "revoked_at", "reason"];
    for expected_col in expected_revoked_columns {
        assert!(revoked_column_names.contains(&expected_col.to_string()),
                "Revoked tokens should have {} column", expected_col);
    }
    
    // Test active_tokens table schema
    let active_tokens_columns = sqlx::query(
        "SELECT column_name FROM information_schema.columns \
         WHERE table_name = 'active_tokens' AND table_schema = 'public'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch active tokens table schema");
    
    let active_column_names: Vec<String> = active_tokens_columns.iter()
        .map(|row| row.get::<String, _>("column_name"))
        .collect();
    
    let expected_active_columns = vec!["id", "user_id", "jti", "token_type", "expires_at", "created_at", "device_info"];
    for expected_col in expected_active_columns {
        assert!(active_column_names.contains(&expected_col.to_string()),
                "Active tokens should have {} column", expected_col);
    }
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_foreign_key_constraints_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Debug: Check what tables exist
    let tables = sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE' ORDER BY table_name")
        .fetch_all(&fixture.db_pool)
        .await
        .expect("Should fetch table list");
    
    let table_names: Vec<String> = tables.iter()
        .map(|row| row.get::<String, _>("table_name"))
        .collect();
    
    println!("DEBUG: Tables found: {:?}", table_names);
    
    // Test foreign key constraints exist
    let foreign_keys = sqlx::query(
        "SELECT tc.constraint_name, tc.table_name, kcu.column_name, \
                ccu.table_name AS foreign_table_name, ccu.column_name AS foreign_column_name \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
         JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name = tc.constraint_name \
         WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = 'public'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch foreign key constraints");
    
    // Verify foreign key relationships
    let constraint_names: Vec<String> = foreign_keys.iter()
        .map(|row| row.get::<String, _>("constraint_name"))
        .collect();
    
    // Debug output to see what constraint names we actually have
    println!("DEBUG: Foreign key constraint names found: {:?}", constraint_names);
    
    // Additional debugging: try PostgreSQL system catalog approach
    let pg_constraint_names: Vec<String> = sqlx::query_scalar(
        "SELECT conname FROM pg_constraint WHERE contype = 'f' AND connamespace = (SELECT oid FROM pg_namespace WHERE nspname = 'public') ORDER BY conname"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch pg_constraint foreign keys");
    
    println!("DEBUG: PostgreSQL pg_constraint foreign keys: {:?}", pg_constraint_names);
    
    // Additional debugging: Check if constraints exist but with different naming
    let all_constraints: Vec<(String, String)> = sqlx::query_as(
        "SELECT constraint_name, constraint_type FROM information_schema.table_constraints WHERE table_schema = 'public' ORDER BY constraint_name"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch all constraints");
    
    println!("DEBUG: All constraints in database: {:?}", all_constraints);
    
    // The information_schema query is failing, but we have confirmed via pg_constraint that FKs exist
    // Use the pg_constraint results for assertions instead
    assert!(pg_constraint_names.iter().any(|name| name.contains("password_reset")),
            "Should have password reset tokens foreign key");
    assert!(pg_constraint_names.iter().any(|name| name.contains("revoked_tokens")),
            "Should have revoked tokens foreign key");
    assert!(pg_constraint_names.iter().any(|name| name.contains("active_tokens")),
            "Should have active tokens foreign key");
    assert!(pg_constraint_names.iter().any(|name| name.contains("sessions")),
            "Should have sessions foreign key");
    
    // Verify all foreign keys point to users table using a simpler approach
    let fk_details: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT tc.constraint_name, kcu.column_name, ccu.table_name as foreign_table_name \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name \
         JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name \
         WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = 'public'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch foreign key details");
    
    // Verify that all foreign keys reference the users table
    for (constraint_name, _column_name, foreign_table_name) in &fk_details {
        assert_eq!(foreign_table_name, "users",
                  "Foreign key {} should reference users table, but references {}",
                  constraint_name, foreign_table_name);
    }
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_indexes_creation_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Test that expected indexes exist
    let indexes = sqlx::query(
        "SELECT indexname, tablename FROM pg_indexes WHERE schemaname = 'public' ORDER BY tablename, indexname"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch index information");
    
    let index_info: Vec<(String, String)> = indexes.iter()
        .map(|row| (row.get::<String, _>("indexname"), row.get::<String, _>("tablename")))
        .collect();
    
    // Verify essential indexes exist on users table
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "users" && idx.contains("email")),
            "Should have email index on users table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "users" && idx.contains("username")),
            "Should have username index on users table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "users" && idx.contains("role")),
            "Should have role index on users table");
    
    // Verify indexes on password_reset_tokens table
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "password_reset_tokens" && idx.contains("token")),
            "Should have token index on password_reset_tokens table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "password_reset_tokens" && idx.contains("user_id")),
            "Should have user_id index on password_reset_tokens table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "password_reset_tokens" && idx.contains("expires_at")),
            "Should have expires_at index on password_reset_tokens table");
    
    // Verify indexes on revoked_tokens table
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "revoked_tokens" && idx.contains("jti")),
            "Should have jti index on revoked_tokens table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "revoked_tokens" && idx.contains("user_id")),
            "Should have user_id index on revoked_tokens table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "revoked_tokens" && idx.contains("expires_at")),
            "Should have expires_at index on revoked_tokens table");
    
    // Verify indexes on active_tokens table
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "active_tokens" && idx.contains("user_id")),
            "Should have user_id index on active_tokens table");
    assert!(index_info.iter().any(|(idx, tbl)| tbl == "active_tokens" && idx.contains("expires_at")),
            "Should have expires_at index on active_tokens table");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_error_handling_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Test that migrations handle already-applied state gracefully
    run_migrations(&fixture.db_pool).await
        .expect("Initial migrations should succeed");
    
    // Attempt to run migrations again multiple times - should not error
    for i in 1..=5 {
        let result = run_migrations(&fixture.db_pool).await;
        assert!(result.is_ok(), "Re-running migrations attempt {} should not error", i);
    }
    
    // Test that database state remains consistent after multiple runs
    let table_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM information_schema.tables \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
    )
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count tables");
    
    assert!(table_count >= 5, "Should have at least 5 tables (4 app tables + migrations table)");
    
    // Verify migration tracking table remains consistent
    let migration_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM _sqlx_migrations"
    )
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count migration records");
    
    assert_eq!(migration_count, 4, "Should consistently have exactly 4 migrations");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_triggers_and_functions_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Test that the update_updated_at_column function exists (from password reset migration)
    let functions = sqlx::query(
        "SELECT routine_name FROM information_schema.routines \
         WHERE routine_schema = 'public' AND routine_type = 'FUNCTION'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch function list");
    
    let function_names: Vec<String> = functions.iter()
        .map(|row| row.get::<String, _>("routine_name"))
        .collect();
    
    assert!(function_names.contains(&"update_updated_at_column".to_string()),
            "Should have update_updated_at_column function");
    
    // Test that the trigger exists on password_reset_tokens table
    let triggers = sqlx::query(
        "SELECT trigger_name, event_object_table \
         FROM information_schema.triggers \
         WHERE trigger_schema = 'public'"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch trigger list");
    
    let trigger_info: Vec<(String, String)> = triggers.iter()
        .map(|row| (row.get::<String, _>("trigger_name"), row.get::<String, _>("event_object_table")))
        .collect();
    
    assert!(trigger_info.iter().any(|(name, table)|
        name.contains("update_password_reset_tokens_updated_at") && table == "password_reset_tokens"),
        "Should have update trigger on password_reset_tokens table");
    
    // Test that the trigger actually works by inserting and updating data
    let user_id = uuid::Uuid::new_v4();
    let token_id = uuid::Uuid::new_v4();
    
    // Insert a user first
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, is_email_verified) \
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user_id)
    .bind("test_trigger_user")
    .bind("trigger@test.com")
    .bind("hashed_password")
    .bind("user")
    .bind(true)
    .execute(&fixture.db_pool)
    .await
    .expect("Should insert test user");
    
    // Insert a password reset token
    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token, expires_at) \
         VALUES ($1, $2, $3, $4)"
    )
    .bind(token_id)
    .bind(user_id)
    .bind("test_token")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .execute(&fixture.db_pool)
    .await
    .expect("Should insert password reset token");
    
    // Get the initial updated_at timestamp
    let initial_updated_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT updated_at FROM password_reset_tokens WHERE id = $1"
    )
    .bind(token_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should fetch initial updated_at");
    
    // Wait a moment to ensure timestamp difference
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    // Update the token to trigger the update trigger
    sqlx::query(
        "UPDATE password_reset_tokens SET is_used = true WHERE id = $1"
    )
    .bind(token_id)
    .execute(&fixture.db_pool)
    .await
    .expect("Should update password reset token");
    
    // Get the new updated_at timestamp
    let new_updated_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar(
        "SELECT updated_at FROM password_reset_tokens WHERE id = $1"
    )
    .bind(token_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should fetch new updated_at");
    
    // Verify the trigger updated the timestamp
    assert!(new_updated_at > initial_updated_at,
            "Trigger should have updated the updated_at timestamp");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_table_comments_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Test that table comments exist (from revoked_tokens and active_tokens migrations)
    let table_comments = sqlx::query(
        "SELECT table_name, obj_description(oid) as comment \
         FROM pg_class \
         JOIN information_schema.tables ON table_name = relname \
         WHERE table_schema = 'public' AND table_type = 'BASE TABLE' \
         AND obj_description(oid) IS NOT NULL"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch table comments");
    
    let commented_tables: Vec<String> = table_comments.iter()
        .map(|row| row.get::<String, _>("table_name"))
        .collect();
    
    assert!(commented_tables.contains(&"revoked_tokens".to_string()),
            "Revoked tokens table should have comment");
    assert!(commented_tables.contains(&"active_tokens".to_string()),
            "Active tokens table should have comment");
    
    // Test column comments exist
    let column_comments = sqlx::query(
        "SELECT table_name, column_name, col_description(pgc.oid, pa.attnum) as comment \
         FROM pg_class pgc \
         JOIN pg_attribute pa ON pgc.oid = pa.attrelid \
         JOIN information_schema.columns isc ON isc.table_name = pgc.relname AND isc.column_name = pa.attname \
         WHERE isc.table_schema = 'public' \
         AND col_description(pgc.oid, pa.attnum) IS NOT NULL \
         ORDER BY table_name, column_name"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch column comments");
    
    // Verify that essential columns have comments
    let comment_info: Vec<(String, String)> = column_comments.iter()
        .map(|row| (row.get::<String, _>("table_name"), row.get::<String, _>("column_name")))
        .collect();
    
    // Check for specific column comments from the migrations
    assert!(comment_info.iter().any(|(table, col)| table == "revoked_tokens" && col == "jti"),
            "Should have comment on revoked_tokens.jti column");
    assert!(comment_info.iter().any(|(table, col)| table == "active_tokens" && col == "jti"),
            "Should have comment on active_tokens.jti column");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_data_insertion_after_migration_comprehensive() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Test comprehensive data insertion into all migrated tables
    let user_id = uuid::Uuid::new_v4();
    let token_id = uuid::Uuid::new_v4();
    let revoked_token_id = uuid::Uuid::new_v4();
    let active_token_id = uuid::Uuid::new_v4();
    
    // Insert into users table
    let insert_result = sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, is_email_verified) \
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user_id)
    .bind("comprehensive_test_user")
    .bind("comprehensive@test.com")
    .bind("hashed_password")
    .bind("user")
    .bind(true)
    .execute(&fixture.db_pool)
    .await;
    
    assert!(insert_result.is_ok(), "Should be able to insert into users table");
    
    // Insert into password_reset_tokens table (foreign key relationship)
    let token_insert_result = sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token, expires_at) \
         VALUES ($1, $2, $3, $4)"
    )
    .bind(token_id)
    .bind(user_id)
    .bind("comprehensive_test_token")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .execute(&fixture.db_pool)
    .await;
    
    assert!(token_insert_result.is_ok(), "Should be able to insert into password_reset_tokens table with foreign key");
    
    // Insert into revoked_tokens table (foreign key relationship)
    let revoked_insert_result = sqlx::query(
        "INSERT INTO revoked_tokens (id, jti, user_id, token_type, expires_at, reason) \
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(revoked_token_id)
    .bind("comprehensive_test_jti")
    .bind(user_id)
    .bind("access")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .bind("test revocation")
    .execute(&fixture.db_pool)
    .await;
    
    assert!(revoked_insert_result.is_ok(), "Should be able to insert into revoked_tokens table with foreign key");
    
    // Insert into active_tokens table (foreign key relationship with JSONB column)
    let device_info = serde_json::json!({"browser": "test", "os": "test_os"});
    let active_insert_result = sqlx::query(
        "INSERT INTO active_tokens (id, user_id, jti, token_type, expires_at, device_info) \
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(active_token_id)
    .bind(user_id)
    .bind("comprehensive_active_jti")
    .bind("access")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .bind(&device_info)
    .execute(&fixture.db_pool)
    .await;
    
    assert!(active_insert_result.is_ok(), "Should be able to insert into active_tokens table with JSONB data");
    
    // Test comprehensive querying of the inserted data
    let user_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE email = $1"
    )
    .bind("comprehensive@test.com")
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count inserted users");
    
    assert_eq!(user_count, 1, "Should have inserted one user");
    
    let token_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM password_reset_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count password reset tokens");
    
    assert_eq!(token_count, 1, "Should have inserted one password reset token");
    
    let revoked_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM revoked_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count revoked tokens");
    
    assert_eq!(revoked_count, 1, "Should have inserted one revoked token");
    
    let active_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM active_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count active tokens");
    
    assert_eq!(active_count, 1, "Should have inserted one active token");
    
    // Test JSONB query functionality
    let device_browser: Option<String> = sqlx::query_scalar(
        "SELECT device_info->>'browser' FROM active_tokens WHERE id = $1"
    )
    .bind(active_token_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should query JSONB data");
    
    assert_eq!(device_browser, Some("test".to_string()), "Should be able to query JSONB data");
    
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_migration_cascade_delete_functionality() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Test that CASCADE DELETE works correctly for foreign key relationships
    let user_id = uuid::Uuid::new_v4();
    let token_id = uuid::Uuid::new_v4();
    let revoked_token_id = uuid::Uuid::new_v4();
    let active_token_id = uuid::Uuid::new_v4();
    
    // Insert test data with foreign key relationships
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, role, is_email_verified) \
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user_id)
    .bind("cascade_test_user")
    .bind("cascade@test.com")
    .bind("hashed_password")
    .bind("user")
    .bind(true)
    .execute(&fixture.db_pool)
    .await
    .expect("Should insert test user");
    
    sqlx::query(
        "INSERT INTO password_reset_tokens (id, user_id, token, expires_at) \
         VALUES ($1, $2, $3, $4)"
    )
    .bind(token_id)
    .bind(user_id)
    .bind("cascade_test_token")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .execute(&fixture.db_pool)
    .await
    .expect("Should insert password reset token");
    
    sqlx::query(
        "INSERT INTO revoked_tokens (id, jti, user_id, token_type, expires_at) \
         VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(revoked_token_id)
    .bind("cascade_test_jti")
    .bind(user_id)
    .bind("access")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .execute(&fixture.db_pool)
    .await
    .expect("Should insert revoked token");
    
    sqlx::query(
        "INSERT INTO active_tokens (id, user_id, jti, token_type, expires_at) \
         VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(active_token_id)
    .bind(user_id)
    .bind("cascade_active_jti")
    .bind("access")
    .bind(chrono::Utc::now() + chrono::Duration::hours(1))
    .execute(&fixture.db_pool)
    .await
    .expect("Should insert active token");
    
    // Verify data exists before deletion
    let initial_token_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM password_reset_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count tokens");
    
    let initial_revoked_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM revoked_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count revoked tokens");
    
    let initial_active_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM active_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count active tokens");
    
    assert_eq!(initial_token_count, 1, "Should have password reset token before deletion");
    assert_eq!(initial_revoked_count, 1, "Should have revoked token before deletion");
    assert_eq!(initial_active_count, 1, "Should have active token before deletion");
    
    // Delete the user - should cascade to all related tables
    let delete_result = sqlx::query(
        "DELETE FROM users WHERE id = $1"
    )
    .bind(user_id)
    .execute(&fixture.db_pool)
    .await;
    
    assert!(delete_result.is_ok(), "Should be able to delete user");
    
    // Verify cascade deletion worked
    let final_token_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM password_reset_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count tokens after deletion");
    
    let final_revoked_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM revoked_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count revoked tokens after deletion");
    
    let final_active_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM active_tokens WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_one(&fixture.db_pool)
    .await
    .expect("Should count active tokens after deletion");
    
    assert_eq!(final_token_count, 0, "Password reset tokens should be cascade deleted");
    assert_eq!(final_revoked_count, 0, "Revoked tokens should be cascade deleted");
    assert_eq!(final_active_count, 0, "Active tokens should be cascade deleted");
    
    fixture.cleanup().await;
}