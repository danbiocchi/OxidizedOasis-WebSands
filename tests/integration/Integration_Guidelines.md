# Integration Test Guidelines

## Overview

This document provides standardized guidelines for writing effective, consistent, and maintainable integration tests within our testing infrastructure. These guidelines ensure all integration tests follow the same patterns, reducing errors and improving maintainability.

## File Structure and Organization

### Test File Naming
- Use descriptive names ending with `_tests.rs`
- Group related functionality: `admin_user_management_tests.rs`, `api_handler_tests.rs`
- Follow the pattern: `{domain}_{functionality}_tests.rs`

### Test Module Organization
```rust
// tests/integration/example_tests.rs

// Import shared utilities first
use test_common::{
    create_test_app, create_test_user, generate_admin_token,
    TestFixtures, MockEmailService
};
use oxidizedoasis_websands::common::error::ApiErrorType;

// Organize tests in logical modules
mod authentication_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_login_success() {
        // Test implementation
    }
}

mod authorization_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_admin_only_endpoint() {
        // Test implementation
    }
}

mod error_handling_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_invalid_request_format() {
        // Test implementation
    }
}
```

## Standard Test Structure

### Test Function Template
```rust
#[tokio::test]
async fn test_{what_is_being_tested}_{expected_outcome}() {
    // 1. ARRANGE - Set up test data and environment
    let app = create_test_app().await;
    let test_user = create_test_user().await;
    let token = generate_admin_token(&test_user.id).await;
    
    // 2. ACT - Perform the action being tested
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/endpoint")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&request_payload)
            .to_request(),
    ).await;
    
    // 3. ASSERT - Verify the results
    assert_eq!(response.status(), StatusCode::OK);
    let body: ApiResponse<ExpectedType> = test::read_body_json(response).await;
    assert!(body.success);
    assert_eq!(body.data.field, expected_value);
}
```

## Required Imports

### Standard Imports for All Integration Tests
```rust
use test_common::{
    // App creation
    create_test_app,
    
    // User management
    create_test_user, create_admin_user, create_unverified_user,
    
    // Token generation
    generate_admin_token, generate_user_token, generate_expired_token,
    
    // Test fixtures
    TestFixtures,
    
    // Mock services
    MockEmailService, MockUserRepository,
};

// Application types
use oxidizedoasis_websands::{
    common::error::{ApiError, ApiErrorType},
    api::responses::user_response::ApiResponse,
    core::user::model::{User, UserResponse},
};

// HTTP testing
use actix_web::{test, http::StatusCode, App};
use serde_json::json;
```

## Authentication Testing Patterns

### Testing Public Endpoints
```rust
#[tokio::test]
async fn test_public_endpoint_no_auth_required() {
    let app = create_test_app().await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/public/health")
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

### Testing Protected Endpoints
```rust
#[tokio::test]
async fn test_protected_endpoint_with_valid_token() {
    let app = create_test_app().await;
    let user = create_test_user().await;
    let token = generate_user_token(&user.id).await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/profile")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protected_endpoint_without_token() {
    let app = create_test_app().await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/profile")
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
```

### Testing Admin Endpoints
```rust
#[tokio::test]
async fn test_admin_endpoint_with_admin_token() {
    let app = create_test_app().await;
    let admin = create_admin_user().await;
    let token = generate_admin_token(&admin.id).await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/users")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_admin_endpoint_with_user_token() {
    let app = create_test_app().await;
    let user = create_test_user().await;
    let token = generate_user_token(&user.id).await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/admin/users")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
```

## Request and Response Testing

### Testing POST Requests with JSON
```rust
#[tokio::test]
async fn test_create_resource_success() {
    let app = create_test_app().await;
    let admin = create_admin_user().await;
    let token = generate_admin_token(&admin.id).await;
    
    let request_payload = json!({
        "name": "Test Resource",
        "description": "A test resource for validation"
    });
    
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/resources")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .insert_header(("Content-Type", "application/json"))
            .set_json(&request_payload)
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::CREATED);
    
    let body: ApiResponse<ResourceResponse> = test::read_body_json(response).await;
    assert!(body.success);
    assert_eq!(body.data.name, "Test Resource");
}
```

### Testing Validation Errors
```rust
#[tokio::test]
async fn test_create_resource_validation_error() {
    let app = create_test_app().await;
    let admin = create_admin_user().await;
    let token = generate_admin_token(&admin.id).await;
    
    let invalid_payload = json!({
        "name": "", // Empty name should fail validation
        "description": "Valid description"
    });
    
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/resources")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(&invalid_payload)
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    let body: ApiResponse<()> = test::read_body_json(response).await;
    assert!(!body.success);
    assert!(body.message.contains("validation"));
}
```

## Error Testing Standards

### Test All Error Scenarios
For each endpoint, test:
1. **Success case** - Valid request with proper authentication
2. **Authentication failure** - Missing or invalid token
3. **Authorization failure** - Valid token but insufficient permissions
4. **Validation failure** - Invalid request data
5. **Not found** - Valid request for non-existent resource
6. **Conflict** - Request that violates business rules

### Error Response Validation
```rust
#[tokio::test]
async fn test_resource_not_found() {
    let app = create_test_app().await;
    let user = create_test_user().await;
    let token = generate_user_token(&user.id).await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/resources/non-existent-id")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    
    let body: ApiResponse<()> = test::read_body_json(response).await;
    assert!(!body.success);
    assert!(body.message.contains("not found"));
}
```

## Database Testing Guidelines

### Use Test Fixtures
```rust
#[tokio::test]
async fn test_user_operations_with_db() {
    let app = create_test_app().await;
    
    // Use predefined test fixtures
    let test_user = TestFixtures::create_verified_user().await;
    let admin_user = TestFixtures::create_admin_user().await;
    
    // Perform test operations
    // ...
    
    // Cleanup is handled automatically by test infrastructure
}
```

### Avoid Hardcoded IDs
```rust
// ❌ Bad - hardcoded UUID
let user_id = "123e4567-e89b-12d3-a456-426614174000";

// ✅ Good - use dynamic test data
let user = create_test_user().await;
let user_id = user.id;
```

## Mock Service Testing

### Using Mock Email Service
```rust
#[tokio::test]
async fn test_registration_sends_verification_email() {
    let mut mock_email_service = MockEmailService::new();
    
    // Set up mock expectations
    mock_email_service
        .expect_send_verification_email()
        .times(1)
        .returning(|_, _| Ok(()));
    
    let app = create_test_app_with_mocks(mock_email_service).await;
    
    // Test registration
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&registration_payload)
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::CREATED);
    // Mock expectations are automatically verified
}
```

## Test Naming Conventions

### Function Names
- Use descriptive names: `test_{action}_{scenario}_{expected_outcome}`
- Examples:
  - `test_login_user_success`
  - `test_login_user_invalid_credentials`
  - `test_create_user_missing_required_fields`
  - `test_delete_user_unauthorized_access`

### Module Names
- Group related tests: `authentication_tests`, `authorization_tests`
- Use descriptive module names: `user_management_tests`, `password_reset_tests`

## Common Anti-Patterns to Avoid

### ❌ Don't Use Shared Mutable State
```rust
// Bad - shared state between tests
static mut GLOBAL_COUNTER: i32 = 0;

#[tokio::test]
async fn test_something() {
    unsafe {
        GLOBAL_COUNTER += 1; // Can cause race conditions
    }
}
```

### ❌ Don't Skip Cleanup
```rust
// Bad - leaving test data
#[tokio::test]
async fn test_create_user() {
    let user = create_user_in_db().await;
    // Missing cleanup - user remains in database
}
```

### ❌ Don't Use Sleep for Timing
```rust
// Bad - unreliable timing
#[tokio::test]
async fn test_async_operation() {
    start_operation();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(operation_completed()); // May be flaky
}
```

### ❌ Don't Test Implementation Details
```rust
// Bad - testing internal implementation
#[tokio::test]
async fn test_password_hashing_uses_bcrypt() {
    // Should test behavior, not implementation
}

// Good - test behavior
#[tokio::test]
async fn test_password_validation_accepts_correct_password() {
    // Test the observable behavior
}
```

## Performance Considerations

### Parallel Test Execution
- Tests run in parallel by default
- Ensure test isolation to prevent race conditions
- Use unique test data for each test

### Database Connections
- Use connection pooling provided by `test_common`
- Don't create unnecessary database connections
- Let the test infrastructure manage connection lifecycle

### Test Data Size
- Keep test data minimal but representative
- Use factories for creating test data consistently
- Clean up large test data sets

## Documentation Standards

### Test Comments
```rust
/// Tests that user registration creates a new user account and sends a verification email.
/// 
/// This test verifies:
/// - User account is created with correct data
/// - Password is properly hashed
/// - Verification email is sent
/// - Appropriate HTTP status is returned
#[tokio::test]
async fn test_user_registration_success() {
    // Implementation
}
```

### Module Documentation
```rust
//! User authentication integration tests.
//! 
//! These tests verify the complete authentication flow including:
//! - User registration and email verification
//! - Login and token generation
//! - Password reset functionality
//! - Token refresh and logout

mod user_authentication_tests {
    // Tests
}
```

## Checklist for New Integration Tests

Before submitting new integration tests, ensure:

- [ ] Test follows the standard file naming convention
- [ ] All required imports are included
- [ ] Test uses proper authentication patterns
- [ ] Both success and failure scenarios are tested
- [ ] Error responses are validated
- [ ] Test data is created using fixtures
- [ ] No hardcoded values are used
- [ ] Test is properly documented
- [ ] Test runs in isolation (no shared state)
- [ ] Cleanup is handled automatically
- [ ] Performance considerations are addressed