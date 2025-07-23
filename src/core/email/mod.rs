// src/core/email/mod.rs
pub mod service; // Made public
pub mod templates; // Also make public if needed by other modules, or keep private if not

pub use service::EmailServiceTrait;

// Re-export mock trait for testing
#[cfg(any(test, feature = "test-utils"))]
pub use service::MockEmailServiceTrait;
