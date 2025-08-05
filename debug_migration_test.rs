// Debug script to see what's actually in the database
use sqlx::Row;
use test_common::UnifiedTestFixture;
use oxidizedoasis_websands::infrastructure::database::run_migrations;

#[tokio::main]
async fn main() {
    let fixture = UnifiedTestFixture::new_with_database().await;
    
    // Run migrations
    run_migrations(&fixture.db_pool).await
        .expect("Migrations should run successfully");
    
    // Check migration descriptions
    println!("=== Migration Descriptions ===");
    let migrations = sqlx::query(
        "SELECT version, description FROM _sqlx_migrations ORDER BY version"
    )
    .fetch_all(&fixture.db_pool)
    .await
    .expect("Should fetch migration records");
    
    for migration in &migrations {
        let version: i64 = migration.get("version");
        let description: String = migration.get("description");
        println!("Version: {}, Description: '{}'", version, description);
    }
    
    // Check foreign key constraint names
    println!("\n=== Foreign Key Constraints ===");
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
    
    for fk in &foreign_keys {
        let constraint_name: String = fk.get("constraint_name");
        let table_name: String = fk.get("table_name");
        println!("Constraint: '{}', Table: '{}'", constraint_name, table_name);
    }
    
    fixture.cleanup().await;
}