//! Hugging Face documentation provider for LLM and ML model documentation.
//!
//! Provides access to:
//! - Transformers library documentation
//! - Swift Transformers for iOS/macOS
//! - Model Hub documentation
//! - Tokenizers library

pub mod client;
pub mod types;

pub use client::HuggingFaceClient;
pub use types::*;
