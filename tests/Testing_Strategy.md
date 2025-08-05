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
- **Location**: Co-located with source code using `#[cfg(test)]` modules
- **Scope**: Individual functions and methods
- **Coverage**: 302 tests covering core business logic

### Integration Tests (in `tests/integration/`)
- **Scope**: API endpoints, middleware, and component interactions
- **Database**: Uses real database connections with test isolation
- **Authentication**: Tests with actual JWT tokens and middleware
- **Coverage**: 244 tests across 14 test files
- **Naming Convention**: Integration test files follow the pattern `<source_file>_tests.rs` where `<source_file>` corresponds to the main source file being tested (e.g., `middleware.rs` → `middleware_tests.rs`)

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

## Best Practices

### 1. Test Organization
- Group related tests in the same file using modules
- Use descriptive test names that explain the scenario
- Organize tests by functionality (authentication, user management, admin, etc.)
- Follow the naming convention for integration tests: `<source_file>_tests.rs`

### 2. Database Testing
- Always use isolated test databases
- Clean up test data after each test
- Use transactions for rollback capabilities where appropriate

### 3. Mock Usage
- Use mocks for external dependencies (email, external APIs)
- Keep real integrations for database and core business logic
- Ensure mocks accurately represent real service behavior

### 4. Error Testing
- Test both success and failure scenarios
- Validate error messages and status codes
- Test edge cases and boundary conditions

### 5. Authentication Testing
- Test both authenticated and unauthenticated scenarios
- Validate different user roles and permissions
- Test token expiration and refresh scenarios

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