pub mod connection; // Make the module public
mod migrations;


// Re-export commonly used items for easier access
pub use connection::create_pool;
