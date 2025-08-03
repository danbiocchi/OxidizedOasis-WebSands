# End-to-End (E2E) Testing Guidelines

## Overview

End-to-End testing validates complete user workflows and system integration by testing the application from the user's perspective. These tests ensure that all components work together correctly in realistic scenarios.

## E2E vs Integration Testing

### Integration Tests
- Test individual API endpoints and components
- Use mocked external dependencies
- Focus on component interactions
- Fast execution
- Located in `tests/integration/`

### E2E Tests
- Test complete user workflows
- Use real external dependencies where possible
- Focus on user journeys and business processes
- Slower execution but higher confidence
- Located in `tests/e2e/`

## Directory Structure

```
tests/e2e/
├── E2E_Guidelines.md           # This file
├── scenarios/                  # Complete user workflow tests
│   ├── user_registration_flow.rs
│   ├── password_reset_flow.rs
│   ├── admin_user_management_flow.rs
│   └── authentication_flow.rs
├── browser/                    # Browser automation tests (future)
│   ├── login_ui_tests.rs
│   └── admin_dashboard_tests.rs
└── api/                       # Full API workflow tests
    ├── complete_user_journey.rs
    └── admin_operations_flow.rs
```

## E2E Test Configuration

### Cargo.toml Configuration
```toml
# In the root Cargo.toml
[[test]]
name = "user_registration_flow"
path = "tests/e2e/scenarios/user_registration_flow.rs"

[[test]]
name = "password_reset_flow"
path = "tests/e2e/scenarios/password_reset_flow.rs"

[[test]]
name = "admin_user_management_flow"
path = "tests/e2e/scenarios/admin_user_management_flow.rs"
```

### Test Environment Setup
```rust
// tests/e2e/scenarios/example_flow.rs
use test_common::{
    create_e2e_test_app,  // Uses real database and services
    E2ETestFixtures,
    real_email_service_if_configured,
};
use oxidizedoasis_websands::infrastructure::config::AppConfig;

#[tokio::test]
async fn test_complete_user_workflow() {
    // Setup real environment for E2E testing
    let config = AppConfig::from_env_for_e2e_testing();
    let app = create_e2e_test_app(config).await;
    
    // Run complete workflow
}
```

## Standard E2E Test Patterns

### Complete User Journey Template
```rust
#[tokio::test]
async fn test_complete_user_registration_to_login_flow() {
    // SETUP: Prepare clean environment
    let app = create_e2e_test_app().await;
    let unique_email = format!("test+{}@example.com", uuid::Uuid::new_v4());
    
    // STEP 1: User Registration
    let registration_payload = json!({
        "username": "testuser123",
        "email": unique_email,
        "password": "SecurePassword123!",
        "confirm_password": "SecurePassword123!"
    });
    
    let register_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&registration_payload)
            .to_request(),
    ).await;
    
    assert_eq!(register_response.status(), StatusCode::CREATED);
    
    // STEP 2: Email Verification (simulate clicking email link)
    let user = find_user_by_email(&unique_email).await;
    let verification_token = get_user_verification_token(&user.id).await;
    
    let verify_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/auth/verify-email?token={}", verification_token))
            .to_request(),
    ).await;
    
    assert_eq!(verify_response.status(), StatusCode::OK);
    
    // STEP 3: User Login
    let login_payload = json!({
        "username": "testuser123",
        "password": "SecurePassword123!"
    });
    
    let login_response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/auth/login")
            .set_json(&login_payload)
            .to_request(),
    ).await;
    
    assert_eq!(login_response.status(), StatusCode::OK);
    
    let login_body: ApiResponse<LoginResponse> = test::read_body_json(login_response).await;
    assert!(login_body.success);
    assert!(!login_body.data.access_token.is_empty());
    
    // STEP 4: Access Protected Resource
    let profile_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/profile")
            .insert_header(("Authorization", format!("Bearer {}", login_body.data.access_token)))
            .to_request(),
    ).await;
    
    assert_eq!(profile_response.status(), StatusCode::OK);
    
    let profile_body: ApiResponse<UserResponse> = test::read_body_json(profile_response).await;
    assert_eq!(profile_body.data.email, Some(unique_email));
    
    // CLEANUP: Remove test user
    cleanup_test_user(&user.id).await;
}
```

## Workflow-Based Test Organization

### User Authentication Flow
```rust
//! Complete authentication workflow tests
//! 
//! Tests cover the entire user authentication journey from registration to logout

mod user_authentication_flow {
    use super::*;
    
    #[tokio::test]
    async fn test_new_user_complete_registration_flow() {
        // Registration -> Email verification -> First login -> Profile access
    }
    
    #[tokio::test]
    async fn test_returning_user_login_flow() {
        // Existing user -> Login -> Access resources -> Logout
    }
    
    #[tokio::test]
    async fn test_password_reset_complete_flow() {
        // Forgot password -> Reset email -> New password -> Login with new password
    }
}
```

### Admin Operations Flow
```rust
//! Administrative operations workflow tests
//! 
//! Tests cover complete admin workflows for user and system management

mod admin_operations_flow {
    use super::*;
    
    #[tokio::test]
    async fn test_admin_user_management_workflow() {
        // Admin login -> View users -> Create user -> Update user -> Deactivate user
    }
    
    #[tokio::test]
    async fn test_admin_security_incident_workflow() {
        // Admin login -> Create incident -> Update status -> Resolve incident
    }
    
    #[tokio::test]
    async fn test_admin_system_configuration_workflow() {
        // Admin login -> Update settings -> Verify changes -> Monitor logs
    }
}
```

## Email Integration Testing

### Real Email Service Testing (Optional)
```rust
#[tokio::test]
#[ignore] // Only run when email service is configured
async fn test_email_verification_with_real_service() {
    if !is_email_service_configured() {
        return; // Skip if email not configured
    }
    
    let app = create_e2e_test_app_with_real_email().await;
    
    // Register user
    let registration = register_test_user(&app).await;
    
    // Check real email was sent (requires email service integration)
    let verification_link = get_verification_link_from_email_service().await;
    
    // Follow verification link
    let verify_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&verification_link)
            .to_request(),
    ).await;
    
    assert_eq!(verify_response.status(), StatusCode::OK);
}
```

### Email Mock for Reliable Testing
```rust
#[tokio::test]
async fn test_email_verification_workflow_with_mock() {
    let mut mock_email_service = MockEmailService::new();
    
    // Setup email expectations
    mock_email_service
        .expect_send_verification_email()
        .times(1)
        .returning(|email, token| {
            // Store the verification token for later use
            store_verification_token_for_testing(email, token);
            Ok(())
        });
    
    let app = create_e2e_test_app_with_mocks(mock_email_service).await;
    
    // Complete workflow using stored verification token
    let user_email = "test@example.com";
    register_user(&app, user_email).await;
    
    let verification_token = get_stored_verification_token(user_email);
    verify_email(&app, &verification_token).await;
    
    login_user(&app, user_email).await;
}
```

## Database State Management

### Test Data Lifecycle
```rust
#[tokio::test]
async fn test_user_lifecycle_complete_flow() {
    // CREATE: Start with clean database state
    let initial_user_count = count_users_in_database().await;
    
    // REGISTER: User registration
    let user = register_and_verify_user().await;
    assert_eq!(count_users_in_database().await, initial_user_count + 1);
    
    // UPDATE: User profile updates
    update_user_profile(&user.id, "New Username").await;
    let updated_user = get_user_from_database(&user.id).await;
    assert_eq!(updated_user.username, "New Username");
    
    // DELETE: User account deletion
    delete_user_account(&user.id).await;
    assert_eq!(count_users_in_database().await, initial_user_count);
    
    // VERIFY: Cleanup completed
    assert!(get_user_from_database(&user.id).await.is_none());
}
```

### Database Transaction Testing
```rust
#[tokio::test]
async fn test_transaction_rollback_on_failure() {
    let app = create_e2e_test_app().await;
    let initial_state = capture_database_state().await;
    
    // Perform operation that should fail and rollback
    let response = test::call_service(
        &app,
        test::TestRequest::post()
            .uri("/api/admin/batch-user-update")
            .set_json(&invalid_batch_payload)
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    
    // Verify database state unchanged
    let final_state = capture_database_state().await;
    assert_eq!(initial_state, final_state);
}
```

## Performance and Load Testing

### Basic Performance Validation
```rust
#[tokio::test]
async fn test_user_registration_performance() {
    let app = create_e2e_test_app().await;
    let start_time = std::time::Instant::now();
    
    // Perform registration
    let response = register_user(&app, "performance@test.com").await;
    
    let duration = start_time.elapsed();
    
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(duration < std::time::Duration::from_secs(2), 
           "Registration took too long: {:?}", duration);
}
```

### Concurrent User Simulation
```rust
#[tokio::test]
async fn test_concurrent_user_registrations() {
    let app = create_e2e_test_app().await;
    
    // Create multiple concurrent registration requests
    let mut tasks = Vec::new();
    
    for i in 0..10 {
        let app_clone = app.clone();
        tasks.push(tokio::spawn(async move {
            register_user(&app_clone, &format!("user{}@test.com", i)).await
        }));
    }
    
    // Wait for all registrations to complete
    let results = futures::future::join_all(tasks).await;
    
    // Verify all succeeded
    for result in results {
        let response = result.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    
    // Verify database consistency
    assert_eq!(count_users_matching_pattern("user%@test.com").await, 10);
}
```

## Error Recovery Testing

### Network Failure Simulation
```rust
#[tokio::test]
async fn test_recovery_from_database_connection_failure() {
    let app = create_e2e_test_app().await;
    
    // Simulate database connection failure
    simulate_database_disconnect().await;
    
    let response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/profile")
            .insert_header(("Authorization", "Bearer valid_token"))
            .to_request(),
    ).await;
    
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    
    // Restore connection and verify recovery
    restore_database_connection().await;
    
    let recovery_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/profile")
            .insert_header(("Authorization", "Bearer valid_token"))
            .to_request(),
    ).await;
    
    assert_eq!(recovery_response.status(), StatusCode::OK);
}
```

## Cross-Feature Integration

### Multi-Service Workflow Testing
```rust
#[tokio::test]
async fn test_user_admin_interaction_workflow() {
    let app = create_e2e_test_app().await;
    
    // PART 1: User registers and gets verified
    let user = register_and_verify_user(&app, "user@test.com").await;
    
    // PART 2: Admin reviews and approves user
    let admin = create_admin_user(&app).await;
    let admin_token = generate_admin_token(&admin.id).await;
    
    approve_user_as_admin(&app, &admin_token, &user.id).await;
    
    // PART 3: User can now access premium features
    let user_token = login_user(&app, "user@test.com").await;
    
    let premium_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/premium-features")
            .insert_header(("Authorization", format!("Bearer {}", user_token)))
            .to_request(),
    ).await;
    
    assert_eq!(premium_response.status(), StatusCode::OK);
    
    // PART 4: Admin can see user activity
    let activity_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri(&format!("/api/admin/user-activity/{}", user.id))
            .insert_header(("Authorization", format!("Bearer {}", admin_token)))
            .to_request(),
    ).await;
    
    assert_eq!(activity_response.status(), StatusCode::OK);
}
```

## Security Testing in E2E Context

### Authentication Flow Security
```rust
#[tokio::test]
async fn test_security_authentication_complete_flow() {
    let app = create_e2e_test_app().await;
    
    // 1. Verify password requirements enforced
    let weak_password_response = attempt_registration_with_weak_password(&app).await;
    assert_eq!(weak_password_response.status(), StatusCode::BAD_REQUEST);
    
    // 2. Verify rate limiting on login attempts
    for _ in 0..5 {
        attempt_login_with_wrong_password(&app, "user@test.com").await;
    }
    
    let rate_limited_response = attempt_login_with_wrong_password(&app, "user@test.com").await;
    assert_eq!(rate_limited_response.status(), StatusCode::TOO_MANY_REQUESTS);
    
    // 3. Verify token expiration handling
    let expired_token = generate_expired_token().await;
    let expired_response = test::call_service(
        &app,
        test::TestRequest::get()
            .uri("/api/user/profile")
            .insert_header(("Authorization", format!("Bearer {}", expired_token)))
            .to_request(),
    ).await;
    
    assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);
}
```

## Running E2E Tests

### Environment Setup
```bash
# Set up test environment variables
export DATABASE_URL="postgresql://test_user:test_pass@localhost/e2e_test_db"
export JWT_SECRET="test_secret_key_for_e2e_testing"
export EMAIL_SERVICE_ENABLED="false"  # Use mocks unless testing real email

# Create test database
createdb e2e_test_db
```

### Execution Commands
```bash
# Run all E2E tests
cargo test --test user_registration_flow
cargo test --test password_reset_flow
cargo test --test admin_user_management_flow

# Run with real email service (if configured)
cargo test --test email_integration_flow -- --ignored

# Run with verbose output
cargo test --test user_registration_flow -- --nocapture
```

## E2E Test Checklist

Before submitting E2E tests, ensure:

- [ ] Test covers a complete user workflow
- [ ] All steps in the workflow are validated
- [ ] Test uses realistic data and scenarios
- [ ] Database state is properly managed
- [ ] External dependencies are handled appropriately
- [ ] Error scenarios and recovery are tested
- [ ] Performance characteristics are validated
- [ ] Security aspects are covered
- [ ] Test cleanup is complete
- [ ] Test can run independently
- [ ] Test is documented with workflow steps
- [ ] Test follows naming conventions

## Maintenance Guidelines

### Regular E2E Test Review
- Review E2E tests monthly for relevance
- Update tests when user workflows change
- Remove obsolete workflow tests
- Add tests for new user journeys

### Performance Monitoring
- Monitor E2E test execution time
- Investigate tests taking longer than expected
- Optimize database operations in tests
- Consider parallel execution for independent workflows

### Reliability Improvements
- Make tests more deterministic
- Reduce flaky test occurrences
- Improve error messages and debugging info
- Add retry logic for transient failures where appropriate