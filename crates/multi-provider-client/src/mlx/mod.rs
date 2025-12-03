//! MLX documentation provider for Apple Silicon ML framework.
//!
//! This module provides access to MLX-Swift and MLX Python documentation,
//! enabling AI assistants to help with machine learning on Apple Silicon.

pub mod client;
pub mod types;

pub use client::MlxClient;
pub use types::*;
