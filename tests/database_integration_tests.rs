use crate::common::{
    create_test_db_pool, create_test_new_user,
};
use crate::common::database::DatabaseTestHelper;
use chrono::{Duration, Utc};
use oxidizedoasis_websands::core::user::{User, NewUser};
use sqlx::{Pool, Postgres, Row};
use std::sync::Arc;
use uuid::Uuid;

mod common;

const TEST_EMAIL: &str = "test@example.com";
const TEST_USERNAME: &str = "testuser";

/// Test database connection and basic operations
#[tokio::test]
async fn test_database_connection() {
    let db_pool = create_test_db_pool().await.unwrap();
    let db_helper = DatabaseTestHelper::new(db_pool.clone());
    
    // Test basic connection
    let result = sqlx::query("SELECT 1 as test")
        .fetch_one(db_pool.as_ref())
        .await;
    
    assert!(result.is_ok());
    let row = result.unwrap();
    let test_value: i32 = row.get("test");
    assert_eq!(test_value, 1);
    
    db_helper.cleanup_all().await.unwrap();
}

/// Test user CRUD operations directly on database
#[tokio::test]
async fn test_user_database_operations() {
    let db_pool = create_test_db_pool().await.unwrap();
    let db_helper = DatabaseTestHelper::new(db_pool.clone());
    
    // Test user creation
    let user_data = create_test_new_user(TEST_USERNAME, TEST_EMAIL, false);
    let user_id = Uuid::new_v4();
    
    // Insert user directly into database
    sqlx::query!(
        r#"
        INSERT INTO users (id, username, email, password_hash, is_email_verified, role, is_active, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
        "#,
        user_id,
        user_data.username,
        user_data.email,
        user_data.password_hash,
        user_data.is_email_verified,
        user_data.role,
        true
    )
    .execute(db_pool.as_ref())
    .await
    .unwrap();
    
    // Test find by email
    let found_user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE email = $1",
        user_data.email
    )
    .fetch_one(db_pool.as_ref())
    .await
    .unwrap();
    
    assert_eq!(found_user.id, user_id);
    assert_eq!(found_user.email, user_data.email);
    assert_eq!(found_user.username, user_data.username);
    assert!(!found_user.is_email_verified);
    
    // Test user update
    sqlx::query!(
        "UPDATE users SET is_email_verified = true, username = $1, updated_at = NOW() WHERE id = $2",
        "updated_username",
        user_id
    )
    .execute(db_pool.as_ref())
    .await
    .unwrap();
    
    let updated_user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1",
        user_id
    )
    .fetch_one(db_pool.as_ref())
    .await
    .unwrap();
    
    assert!(updated_user.is_email_verified);
    assert_eq!(updated_user.username, "updated_username");
    
    // Test user deletion
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(db_pool.as_ref())
        .await
        .unwrap();
    
    let deleted_user = sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
        .fetch_optional(db_pool.as_ref())
        .await
        .unwrap();
    
    assert!(deleted_user.is_none());
    
    db_helper.cleanup_all().await.unwrap();
}