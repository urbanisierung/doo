//! # doo - Command Wrapper CLI
//!
//! A powerful CLI tool that wraps other commands with persistent variables and contexts.
//!
//! ## Features
//!
//! - Command wrapping with templates
//! - Persistent variable substitution
//! - Context management for different environments
//! - Interactive terminal menu for command browsing
//! - Cross-platform configuration management
//!
//! ## Usage
//!
//! ```rust
//! use doo::config::ConfigManager;
//! use doo::variables::VariableManager;
//! use doo::context::ContextManager;
//! # use anyhow::Result;
//!
//! # fn main() -> Result<()> {
//! // Initialize the configuration
//! let config_manager = ConfigManager::new()?;
//! let context_manager = ContextManager::new(&config_manager)?;
//! let variable_manager = VariableManager::new(&config_manager)?;
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod context;
pub mod executor;
pub mod interactive;
pub mod variables;

pub use config::{Config, ConfigManager};
pub use context::ContextManager;
pub use executor::CommandExecutor;
pub use interactive::InteractiveMenu;
pub use variables::{Variables, VariableManager};

/// Result type used throughout the crate
pub type Result<T> = anyhow::Result<T>;

/// Error types used in the crate
pub use anyhow::Error;
