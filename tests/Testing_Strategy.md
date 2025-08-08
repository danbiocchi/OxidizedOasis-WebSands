# Testing Strategy

## Overview

Our testing infrastructure is designed for organization, code reuse, and scalability. It moves away from a flat tests directory and organizes tests into distinct suites, while centralizing all shared test logic into a dedicated helper library crate.

## Directory Structure

The project follows a specific layout to separate application code, different types of tests, and shared test utilities:

```
project-root/
├── Cargo.toml                    # Main project configuration with test declarations
├── src/                          # Main application code
│   └── lib.rs
├── migrations/                   # Database migration files
└── tests/
    ├── common/                   # Helper crate for shared test code
    │   ├── Cargo.toml           # Test helper library configuration
    │   └── src/
    │       ├── lib.rs           # Main test utilities export
    │       └── database.rs      # Database test helpers
    ├── integration/             # Integration test suite
    │   ├── admin_logs_tests.rs
    │   ├── admin_security_tests.rs
    │   ├── admin_user_management_tests.rs
    │   ├── api_handler_tests.rs
    │   ├── cors_tests.rs
    │   ├── database_integration_tests.rs
    │   ├── email_service_tests.rs
    │   ├── email_templates_tests.rs
    │   ├── health_check_tests.rs
    │   ├── lib_tests.rs
    │   ├── main_integration_tests.rs
    │   ├── middleware_tests.rs
    │   ├── migrations_tests.rs
    │   ├── user_crud_tests.rs
    │   └── user_tests.rs
    └── e2e/                     # End-to-end test suite (future)
        └── README.md
```

## Environment Configuration

**Important**: This project uses separate environment files for different contexts:
- **`.env`**: Production environment configuration
- **`.env.test`**: Testing environment configuration (used during test execution)

The testing infrastructure automatically loads `.env.test` during test runs, which contains test-specific configurations including database connections, JWT settings, and email configurations. This separation ensures that tests run with appropriate settings isolated from production values.

## Key Components and Configuration

### 1. Test Suite Organization

**Purpose**: To isolate different kinds of tests. Integration tests validate library components and API endpoints, while e2e tests would validate complete user workflows.

**Location**: Each suite resides in its own subdirectory inside `tests/` (e.g., `tests/integration/`).

**Individual Test Files**: Each test file is configured as a separate test target in the root `Cargo.toml`:

```toml
# In the root Cargo.toml
[[test]]
name = "admin_logs_tests"
path = "tests/integration/admin_logs_tests.rs"

[[test]]
name = "admin_security_tests"
path = "tests/integration/admin_security_tests.rs"

[[test]]
name = "api_handler_tests"
path = "tests/integration/api_handler_tests.rs"

# ... and so on for each integration test file
```

### 2. Shared Helper Crate (`tests/common`)

This is the most critical part of the new structure for code reuse.

**Purpose**: Holds all code shared between different test suites, including:
- Database connections and setup/teardown logic
- Mock services (EmailService, UserRepository, etc.)
- Test data fixtures and factories
- Custom assertions and utilities
- JWT token generation for testing
- Common test configuration

**Nature**: A separate, local Rust library crate, not just a module.

**Definition**: Defined by its own `tests/common/Cargo.toml`:

```toml
[package]
name = "test_common"
version = "0.1.0"
edition = "2021"

[dependencies]
oxidizedoasis-websands = { path = "../.." }
actix-web = "4.4"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "chrono", "uuid"] }
tokio = { version = "1.0", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
serde_json = "1.0"
mockall = "0.12"
jsonwebtoken = "9.1"
bcrypt = "0.15"
```

**Integration**: Linked to the main project as a dev-dependency in the root `Cargo.toml`:

```toml
# In the root Cargo.toml
[dev-dependencies]
test_common = { path = "tests/common" }
```

**Usage**: Any test suite imports shared code using standard use statements:

```rust
// In tests/integration/*.rs files
use test_common::{
    create_test_app, create_test_user, generate_admin_token,
    MockEmailService, MockUserRepository, TestFixtures
};

#[tokio::test]
async fn test_something_with_db() {
    let app = create_test_app().await;
    let user = create_test_user().await;
    // ... test logic
}
```

## Test Infrastructure Features

### Database Testing
- **Isolated Test Databases**: Each test gets a unique database to prevent conflicts
- **Migration Management**: Automatic database setup with proper migration paths (`../../migrations`)
- **Connection Pooling**: Efficient database connection management for tests
- **Cleanup**: Automatic teardown of test databases with proper cleanup procedures

### Mock Services
- **Email Service**: Mock implementation for testing email functionality
- **User Repository**: Mock for testing user operations without database
- **JWT Service**: Test token generation and validation utilities

### Test Fixtures
- **User Fixtures**: Predefined test users (admin, regular user, unverified user)
- **Request Fixtures**: Common HTTP request payloads
- **Response Assertions**: Standardized response validation helpers

### Authentication Testing
- **Token Generation**: Helper functions for creating valid test tokens
- **Role-based Testing**: Admin and user token generation utilities
- **CSRF Protection**: Test utilities for CSRF token handling

## Test Types and Coverage

### Unit Tests (in `src/`)
- **Location**: Co-located with source code using `#[cfg(test)]` modules within the same file as the code being tested
- **Scope**: Individual functions and methods
- **Coverage**: 302 tests covering core business logic
- **REQUIREMENT**: Unit tests MUST be embedded in source files, never in separate test directories

### Integration Tests (in `tests/integration/`)
- **Location**: MUST be in `tests/integration/` directory
- **Scope**: API endpoints, middleware, and component interactions
- **Database**: MUST use real database connections with test isolation (never mocks for database operations)
- **Authentication**: Tests with actual JWT tokens and middleware
- **Coverage**: 244 tests across 14 test files
- **Naming Convention**: Integration test files follow the pattern `<source_file>_tests.rs` where `<source_file>` corresponds to the main source file being tested (e.g., `middleware.rs` → `middleware_tests.rs`)
- **REQUIREMENT**: All integration tests MUST be registered in `Cargo.toml` as individual `[[test]]` entries

### End-to-End Tests (in `tests/e2e/`)
- **Location**: MUST be in `tests/e2e/` directory
- **Scope**: Complete user workflows and system interactions
- **Database**: MUST use real database connections
- **REQUIREMENT**: All e2e tests MUST be registered in `Cargo.toml` as individual `[[test]]` entries

### Test Execution Results
- **Unit Tests**: 302 passed, 4 ignored (database connection tests)
- **Integration Tests**: 244 passed across all test files
- **Total**: 546 tests with 100% pass rate

## Test File Naming Conventions

### Integration Tests
Integration test files follow a consistent naming pattern based on the source files they test:

- **Pattern**: `<source_file>_tests.rs`
- **Examples**:
  - `src/infrastructure/middleware/mod.rs` → `tests/integration/middleware_tests.rs`
  - `src/api/handlers/user_handler.rs` → `tests/integration/user_handler_tests.rs`
  - `src/core/email/service.rs` → `tests/integration/email_service_tests.rs`

This naming convention ensures:
- Clear mapping between source files and their tests
- Consistent organization across the test suite
- Easy identification of test coverage gaps

### Unit Tests
Unit tests are co-located with source code using `#[cfg(test)]` modules within the same file as the code being tested.

## Best Practices and Anti-Pattern Prevention

### 1. Test Organization Rules (MANDATORY)
- **Unit Tests**: MUST be embedded in source files using `#[cfg(test)]` modules, never in separate directories
- **Integration Tests**: MUST be in `tests/integration/` directory and registered in `Cargo.toml`
- **E2E Tests**: MUST be in `tests/e2e/` directory and registered in `Cargo.toml`
- Group related tests in the same file using modules
- Use descriptive test names that explain the scenario being tested
- Follow the naming convention for integration tests: `<source_file>_tests.rs`
- **FORBIDDEN**: Never place unit tests in integration test directories

### 2. Database Testing Rules (MANDATORY)
- **Integration Tests**: MUST use real database connections via `UnifiedTestFixture::new_with_database()`
- **E2E Tests**: MUST use real database connections, never mocks
- **FORBIDDEN**: Never use `new_with_mocks()` for integration or e2e tests
- Always use isolated test databases with proper cleanup
- Use transactions for rollback capabilities where appropriate

### 3. Mock Usage Rules
- **ALLOWED**: Use mocks only for external dependencies (email services, external APIs, third-party services)
- **FORBIDDEN**: Never use mocks for database operations in integration or e2e tests
- **FORBIDDEN**: Never use mocks for core business logic in integration tests
- Ensure mocks accurately represent real service behavior when used appropriately

### 4. Code Duplication Prevention (MANDATORY)
- **REQUIREMENT**: Use the `tests/common/` helper crate for all shared test code
- Extract common test setup into helper functions in `tests/common/src/lib.rs`
- **FORBIDDEN**: Duplicate test setup code across multiple test files
- Create reusable helper functions: `create_test_app()`, `create_auth_test_app()`, `create_database_services()`
- Use shared fixtures from the common crate whenever possible

### 5. Assertion Quality Rules (MANDATORY)
- **FORBIDDEN**: Hollow tests that only check HTTP status codes without validating functionality
- **FORBIDDEN**: Useless assertions like `assert!(result.is_ok() || result.is_err())` that always pass
- **FORBIDDEN**: Placeholder tests with only `assert!(true)` or empty test bodies
- **REQUIREMENT**: Every test MUST validate actual functionality and behavior
- **REQUIREMENT**: Assertions MUST be meaningful and capable of failing under incorrect conditions
- Test both success and failure scenarios with specific validation
- Validate error messages, status codes, and response content
- Test edge cases and boundary conditions with proper assertions

### 6. Test Registration Rules (MANDATORY)
- **REQUIREMENT**: All integration tests MUST be declared in root `Cargo.toml` as individual `[[test]]` entries:
  ```toml
  [[test]]
  name = "test_file_name"
  path = "tests/integration/test_file_name.rs"
  ```
- **REQUIREMENT**: All e2e tests MUST be declared in root `Cargo.toml` as individual `[[test]]` entries
- **REQUIREMENT**: Update `Cargo.toml` when adding new test files
- **REQUIREMENT**: Remove `[[test]]` entries when deleting test files

### 7. Authentication Testing Rules
- Test both authenticated and unauthenticated scenarios
- Validate different user roles and permissions
- Test token expiration and refresh scenarios
- Use real JWT tokens in integration tests, not mocked authentication

### 8. Anti-Pattern Detection Checklist

Before committing any test changes, verify:

#### ❌ FORBIDDEN Patterns (Will cause test infrastructure failure):
- [ ] Unit tests placed in `tests/integration/` or `tests/e2e/` directories
- [ ] Integration tests using `new_with_mocks()` instead of `new_with_database()`
- [ ] Hollow tests that only check status codes without validating functionality
- [ ] Useless assertions like `assert!(result.is_ok() || result.is_err())`
- [ ] Placeholder tests with `assert!(true)` or empty bodies
- [ ] Duplicated test setup code across multiple files
- [ ] Test files not registered in `Cargo.toml`
- [ ] Integration tests using mocks for database operations

#### ✅ REQUIRED Patterns (Essential for bulletproof tests):
- [ ] Unit tests embedded in source files with `#[cfg(test)]` modules
- [ ] Integration tests using `UnifiedTestFixture::new_with_database()`
- [ ] All shared test code extracted to `tests/common/` helper crate
- [ ] Meaningful assertions that validate actual functionality
- [ ] Test files properly registered in `Cargo.toml`
- [ ] Real database connections for all integration and e2e tests
- [ ] Descriptive test names that explain the scenario being tested

## Running Tests

### All Tests
```bash
cargo test
```

### Unit Tests Only
```bash
cargo test --lib
```

### Integration Tests Only
```bash
cargo test --test admin_logs_tests
cargo test --test api_handler_tests
# ... or any specific integration test file
```

### With Output
```bash
cargo test -- --nocapture
```

## Migration Path

The current infrastructure replaced a problematic setup where:
1. Integration tests were using a single `main.rs` file with module declarations
2. Import paths were incorrect (trying to access non-existent `oxidizedoasis_websands::test_common`)
3. Database migration paths were wrong (`./migrations` instead of `../../migrations`)
4. Module resolution conflicts prevented proper test execution

The new infrastructure provides:
- Individual test file declarations in `Cargo.toml`
- Proper external crate imports via `test_common`
- Correct relative paths for migrations and resources
- Clean separation between test suites and shared utilities

## Database Cleanup and Maintenance

### Critical Database Management

Our testing infrastructure creates isolated databases for each test to prevent conflicts. However, improper cleanup can lead to serious infrastructure problems, as experienced during development when over 6,069 leftover test databases accumulated, consuming significant disk space and causing PostgreSQL corruption.

### Database Cleanup Infrastructure

#### Automatic Cleanup Functions

The test infrastructure includes automatic cleanup mechanisms in `tests/common/src/lib.rs`:

```rust
pub async fn cleanup_test_database(db_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Terminates active connections and drops test databases
}

pub async fn create_test_config_with_cleanup() -> TestDatabaseConfig {
    // Creates test database with automatic cleanup registration
}
```

#### Database Naming Convention

Test databases follow a specific naming pattern for easy identification:
- Pattern: `test_oxidizedoasis_db_threadid{thread_id}_{sequence}_{timestamp}_{hash}`
- Example: `test_oxidizedoasis_db_threadid32_25_918000_adb1d8f5`

This naming convention allows for:
- Easy identification of test databases
- Automated cleanup scripts
- Thread isolation tracking
- Temporal organization for debugging

### Prevention Strategies

#### 1. Proper Test Design
- **Always use cleanup functions**: Ensure every test that creates a database calls the cleanup function
- **Use test fixtures**: Leverage `create_test_config_with_cleanup()` for automatic cleanup registration
- **Avoid manual database creation**: Use the provided test infrastructure instead of direct SQL commands

#### 2. Infrastructure Monitoring
- **Disk Space Monitoring**: PostgreSQL requires adequate disk space to function properly
- **Database Count Monitoring**: Regularly check for leftover test databases
- **PostgreSQL Health**: Monitor for recovery mode or corruption issues

#### 3. Regular Maintenance

##### Daily/Weekly Checks
```bash
# Check for leftover test databases
psql -d postgres -c "SELECT datname FROM pg_database WHERE datname LIKE 'test_oxidizedoasis_db_%';"

# Monitor disk space
df -h

# Check PostgreSQL status
systemctl status postgresql  # Linux
brew services list | grep postgresql  # macOS
```

##### Cleanup Script Usage

A cleanup script (`cleanup_test_databases.sql`) is available for emergency cleanup:

```sql
-- Generate DROP DATABASE commands for all test databases
SELECT 'DROP DATABASE IF EXISTS "' || datname || '";'
FROM pg_database
WHERE datname LIKE 'test_oxidizedoasis_db_%';
```

### Troubleshooting Database Issues

#### Common Problems and Solutions

1. **"No space left on device" errors**
   - **Cause**: Accumulated test databases consuming disk space
   - **Solution**: Run cleanup script and free disk space with `cargo clean`
   - **Prevention**: Regular monitoring and automated cleanup

2. **PostgreSQL in recovery mode**
   - **Cause**: Database corruption due to disk space issues
   - **Solution**: Restart PostgreSQL service after freeing space
   - **Prevention**: Maintain adequate disk space (>20% free recommended)

3. **Connection refused errors**
   - **Cause**: PostgreSQL service stopped or corrupted
   - **Solution**: Restart PostgreSQL service and verify configuration
   - **Prevention**: Regular health checks and proper shutdown procedures

4. **Test database creation failures**
   - **Cause**: Permission issues or resource constraints
   - **Solution**: Verify PostgreSQL permissions and available connections
   - **Prevention**: Proper connection pooling and resource management

### Emergency Recovery Procedure

If you encounter massive database accumulation (as we did with 6,069+ databases):

1. **Immediate Actions**:
   ```bash
   # Free up space
   cargo clean
   
   # Check available space
   df -h
   ```

2. **Database Cleanup**:
   ```bash
   # Generate cleanup commands
   psql -d postgres -f cleanup_test_databases.sql
   
   # Execute the generated DROP commands
   # (Copy output from above and execute)
   ```

3. **PostgreSQL Recovery**:
   ```bash
   # Restart PostgreSQL
   sudo systemctl restart postgresql  # Linux
   brew services restart postgresql   # macOS
   ```

4. **Verification**:
   ```bash
   # Verify cleanup
   psql -d postgres -c "SELECT COUNT(*) FROM pg_database WHERE datname LIKE 'test_oxidizedoasis_db_%';"
   
   # Run tests to ensure functionality
   cargo test
   ```

### Best Practices Summary

1. **Always use proper cleanup**: Never create test databases without proper cleanup procedures
2. **Monitor regularly**: Check for leftover databases weekly
3. **Maintain disk space**: Keep at least 20% free space for PostgreSQL operations
4. **Use test fixtures**: Leverage the existing test infrastructure for database management
5. **Document issues**: Keep track of any database-related problems for pattern recognition

### Warning Signs

Watch for these indicators of potential database cleanup issues:

- Test execution becoming slower over time
- Disk space warnings
- PostgreSQL connection errors
- Unusual database names in PostgreSQL listings
- Tests failing with "resource unavailable" errors

By following these guidelines and maintaining proper database hygiene, we can prevent the infrastructure problems that caused the original test failures and ensure reliable test execution.

## Test Quality Enforcement and Issue Prevention

### Lessons Learned from Test Infrastructure Fixes

This section documents critical issues discovered during comprehensive test infrastructure review and the mandatory practices to prevent recurrence.

### Critical Issues Fixed and Prevention Measures

#### 1. Hollow Tests (NEVER ALLOW)
**Problem**: Tests that appear functional but only validate superficial aspects like HTTP status codes without testing actual functionality.

**Examples of Hollow Tests** (FORBIDDEN):
```rust
// BAD: Only checks status, not functionality
#[tokio::test]
async fn test_user_creation() {
    let response = create_user_request().await;
    assert_eq!(response.status(), 200); // Hollow - doesn't validate user was actually created
}
```

**Required Pattern**:
```rust
// GOOD: Validates actual functionality
#[tokio::test]
async fn test_user_creation() {
    let app = create_test_app().await;
    let response = create_user_request(&app).await;
    assert_eq!(response.status(), 200);
    
    // REQUIRED: Validate actual functionality
    let user_data: UserResponse = response.json().await;
    assert_eq!(user_data.email, "test@example.com");
    assert!(user_data.id > 0);
    
    // REQUIRED: Verify database state
    let user_in_db = app.user_service.get_user_by_id(user_data.id).await.unwrap();
    assert_eq!(user_in_db.email, "test@example.com");
}
```

#### 2. Useless Assertions (NEVER ALLOW)
**Problem**: Assertions that always pass regardless of the actual result, providing false confidence.

**Examples of Useless Assertions** (FORBIDDEN):
```rust
// BAD: Always passes regardless of actual result
assert!(result.is_ok() || result.is_err()); // Tautological assertion
assert_eq!(true, true); // Meaningless assertion
assert!(true); // Placeholder assertion
```

**Required Pattern**:
```rust
// GOOD: Meaningful assertions that can fail
match result {
    Ok(value) => assert_eq!(value.status, "active"),
    Err(e) => assert_eq!(e.to_string(), "Expected specific error message"),
}
```

#### 3. Mock-Based Integration Tests (NEVER ALLOW)
**Problem**: Integration tests using mocks instead of real database connections, defeating the purpose of integration testing.

**Pattern** (FORBIDDEN):
```rust
// BAD: Integration test using mocks
let fixture = UnifiedTestFixture::new_with_mocks().await;
```

**Required Pattern**:
```rust
// GOOD: Integration test using real database
let fixture = UnifiedTestFixture::new_with_database().await;
```

#### 4. Misplaced Unit Tests (NEVER ALLOW)
**Problem**: Unit tests placed in integration test directories instead of being embedded in source files.

**Required Organization**:
- **Unit Tests**: MUST be in `src/` files with `#[cfg(test)]` modules
- **Integration Tests**: MUST be in `tests/integration/` directory
- **E2E Tests**: MUST be in `tests/e2e/` directory

#### 5. Severe Code Duplication (NEVER ALLOW)
**Problem**: Repeated test setup code across multiple files, making maintenance difficult and error-prone.

**Solution**: MANDATORY use of `tests/common/` helper crate:
```rust
// GOOD: Use shared helper functions
use test_common::{create_test_app, create_auth_test_app, create_database_services};

#[tokio::test]
async fn test_functionality() {
    let app = create_test_app().await; // Shared setup
    // Test logic here
}
```

### Mandatory Review Checklist

Before any test-related changes are merged, verify:

#### Test Organization ✅
- [ ] Unit tests are embedded in source files with `#[cfg(test)]` modules
- [ ] Integration tests are in `tests/integration/` directory
- [ ] E2E tests are in `tests/e2e/` directory
- [ ] All test files are registered in `Cargo.toml`

#### Test Quality ✅
- [ ] No hollow tests that only check status codes
- [ ] No useless assertions that always pass
- [ ] No placeholder tests with `assert!(true)`
- [ ] All tests validate actual functionality and behavior
- [ ] Meaningful test names that describe the scenario

#### Database Usage ✅
- [ ] Integration tests use `new_with_database()` not `new_with_mocks()`
- [ ] E2E tests use real database connections
- [ ] Proper database cleanup is implemented
- [ ] No mocks used for database operations in integration/e2e tests

#### Code Reuse ✅
- [ ] Shared test code extracted to `tests/common/` helper crate
- [ ] No duplicated test setup across files
- [ ] Helper functions used for common operations
- [ ] DRY principle followed throughout test suite

#### Registration ✅
- [ ] New test files added to `Cargo.toml` as `[[test]]` entries
- [ ] Deleted test files removed from `Cargo.toml`
- [ ] Test names match file names in registration

### Success Metrics

A bulletproof test suite demonstrates:
- **100% Pass Rate**: All tests pass consistently
- **Meaningful Validation**: Every test validates actual functionality
- **True Integration**: Integration tests use real database connections
- **Proper Organization**: Unit tests in source files, integration/e2e in test directories
- **Code Efficiency**: Minimal duplication through shared helper functions
- **Maintainability**: Clear test structure that's easy to extend and modify

### Enforcement

These guidelines are MANDATORY. Any test changes that violate these patterns will be rejected during code review. The test infrastructure has been carefully designed to prevent the issues that previously caused test failures and must be maintained according to these standards.