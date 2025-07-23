pub mod jwt;          // Make `jwt` public so its contents can be accessed externally
pub mod service;
pub mod token_revocation;
pub mod active_token;

// Re-export only what's needed
pub use service::AuthService;

// Re-export mock traits for testing
#[cfg(any(test, feature = "test-utils"))]
pub use token_revocation::MockTokenRevocationServiceTrait;
#[cfg(any(test, feature = "test-utils"))]
pub use active_token::MockActiveTokenServiceTrait;
