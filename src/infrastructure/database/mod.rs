pub mod connection; // Make the module public
pub mod migrations; // Make migrations module public for testing

// Re-export commonly used items for easier access
pub use connection::create_pool;
pub use migrations::run_migrations; // Re-export run_migrations for easier access
