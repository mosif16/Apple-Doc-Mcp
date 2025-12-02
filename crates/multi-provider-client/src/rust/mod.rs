pub mod client;
pub mod html_parser;
pub mod types;

pub use client::RustClient;
pub use html_parser::{extract_title_from_html, ParsedDocumentation};
pub use types::*;
