//! HTML parser for Rust documentation pages from doc.rust-lang.org and docs.rs
//!
//! Extracts structured information from rustdoc-generated HTML.

use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use super::types::{RustAssociatedType, RustExample, RustItemKind, RustMethodInfo};

/// Parsed documentation from an HTML page
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedDocumentation {
    pub declaration: Option<String>,
    pub documentation: Option<String>,
    pub examples: Vec<RustExample>,
    pub methods: Vec<RustMethodInfo>,
    pub impl_traits: Vec<String>,
    pub associated_types: Vec<RustAssociatedType>,
    pub source_url: Option<String>,
}

/// Parse a rustdoc HTML page and extract structured documentation
pub fn parse_rustdoc_html(html: &str, item_kind: RustItemKind) -> ParsedDocumentation {
    let document = Html::parse_document(html);
    let mut result = ParsedDocumentation::default();

    // Extract declaration from various possible selectors
    result.declaration = extract_declaration(&document);

    // Extract main documentation
    result.documentation = extract_documentation(&document);

    // Extract code examples
    result.examples = extract_examples(&document);

    // Extract methods for structs/enums/traits
    if matches!(
        item_kind,
        RustItemKind::Struct | RustItemKind::Enum | RustItemKind::Trait | RustItemKind::Union
    ) {
        result.methods = extract_methods(&document);
        result.impl_traits = extract_impl_traits(&document);
    }

    // Extract associated types for traits
    if item_kind == RustItemKind::Trait {
        result.associated_types = extract_associated_types(&document);
    }

    // Extract source URL
    result.source_url = extract_source_url(&document);

    result
}

/// Extract the type/function declaration
fn extract_declaration(document: &Html) -> Option<String> {
    // Try multiple selectors for different rustdoc versions

    // Modern rustdoc uses .item-decl
    let item_decl_selector = Selector::parse(".item-decl pre, .item-decl code").ok()?;
    if let Some(element) = document.select(&item_decl_selector).next() {
        let text = clean_text(&element.text().collect::<String>());
        if !text.is_empty() {
            return Some(text);
        }
    }

    // Older rustdoc uses .rust or .rust-example-rendered in decl
    let decl_selector = Selector::parse(".decl pre, .decl code").ok()?;
    if let Some(element) = document.select(&decl_selector).next() {
        let text = clean_text(&element.text().collect::<String>());
        if !text.is_empty() {
            return Some(text);
        }
    }

    // Function signature
    let fn_selector = Selector::parse(".fn-signature, .fnname").ok()?;
    if let Some(element) = document.select(&fn_selector).next() {
        let text = clean_text(&element.text().collect::<String>());
        if !text.is_empty() {
            return Some(text);
        }
    }

    None
}

/// Extract the main documentation text
fn extract_documentation(document: &Html) -> Option<String> {
    // Main docblock
    let docblock_selector = Selector::parse(".docblock").ok()?;

    let mut doc_parts = Vec::new();

    for docblock in document.select(&docblock_selector).take(3) {
        // Skip if this block has example-related classes
        let self_classes = docblock.value().attr("class").unwrap_or_default();
        if self_classes.contains("example") || self_classes.contains("impl-items") {
            continue;
        }

        let text = extract_text_preserving_structure(&docblock);
        if !text.is_empty() {
            doc_parts.push(text);
        }
    }

    if doc_parts.is_empty() {
        None
    } else {
        Some(doc_parts.join("\n\n"))
    }
}

/// Extract text while preserving some structure (paragraphs, lists)
fn extract_text_preserving_structure(element: &scraper::ElementRef) -> String {
    let mut result = String::new();

    for child in element.children() {
        if let Some(element_ref) = scraper::ElementRef::wrap(child) {
            let tag = element_ref.value().name();
            match tag {
                "p" => {
                    result.push_str(&clean_text(&element_ref.text().collect::<String>()));
                    result.push_str("\n\n");
                }
                "ul" | "ol" => {
                    let li_selector = Selector::parse("li").unwrap();
                    for (i, li) in element_ref.select(&li_selector).enumerate() {
                        let bullet = if tag == "ol" {
                            format!("{}. ", i + 1)
                        } else {
                            "• ".to_string()
                        };
                        result.push_str(&bullet);
                        result.push_str(&clean_text(&li.text().collect::<String>()));
                        result.push('\n');
                    }
                    result.push('\n');
                }
                "pre" | "code" => {}
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    result.push_str("\n## ");
                    result.push_str(&clean_text(&element_ref.text().collect::<String>()));
                    result.push_str("\n\n");
                }
                "div" => {
                    // Recursively process divs
                    let inner = extract_text_preserving_structure(&element_ref);
                    if !inner.is_empty() {
                        result.push_str(&inner);
                    }
                }
                _ => {
                    let text = element_ref.text().collect::<String>();
                    if !text.trim().is_empty() {
                        result.push_str(&clean_text(&text));
                        result.push(' ');
                    }
                }
            }
        } else if let Some(text) = child.value().as_text() {
            let text = text.trim();
            if !text.is_empty() {
                result.push_str(text);
                result.push(' ');
            }
        }
    }

    clean_text(&result)
}

/// Extract code examples from the documentation
fn extract_examples(document: &Html) -> Vec<RustExample> {
    let mut examples = Vec::new();

    // Look for example code blocks
    let example_selector =
        Selector::parse(".example-wrap pre, .rust-example-rendered, pre.rust").ok();

    if let Some(selector) = example_selector {
        for element in document.select(&selector).take(5) {
            let code = element.text().collect::<String>();
            let code = clean_code(&code);

            if !code.is_empty() && code.len() > 10 {
                // Find preceding description if any
                let description = element
                    .prev_siblings()
                    .filter_map(scraper::ElementRef::wrap)
                    .find(|e| e.value().name() == "p")
                    .map(|e| clean_text(&e.text().collect::<String>()));

                examples.push(RustExample { code, description });
            }
        }
    }

    examples
}

/// Extract methods from struct/enum/trait documentation
fn extract_methods(document: &Html) -> Vec<RustMethodInfo> {
    let mut methods = Vec::new();

    // Method items in impl blocks
    let method_selector = Selector::parse(
        ".method, .impl-items .toggle:not(.implementors-toggle), .impl-items > details",
    )
    .ok();

    if let Some(selector) = method_selector {
        for element in document.select(&selector).take(50) {
            if let Some(method) = parse_method_element(&element) {
                methods.push(method);
            }
        }
    }

    // Also try .structfield for struct fields
    let field_selector = Selector::parse(".structfield").ok();
    if let Some(selector) = field_selector {
        for element in document.select(&selector).take(50) {
            let name = element
                .select(&Selector::parse(".structfield-name, .field-name, code").unwrap())
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_default();

            if !name.is_empty() {
                let signature = clean_text(&element.text().collect::<String>());

                // Get brief description
                let summary = element
                    .next_siblings()
                    .filter_map(scraper::ElementRef::wrap)
                    .find(|e| e.value().name() == "div" || e.value().name() == "p")
                    .map(|e| clean_text(&e.text().collect::<String>()))
                    .unwrap_or_default();

                methods.push(RustMethodInfo {
                    name: clean_text(&name),
                    signature,
                    summary,
                    is_unsafe: false,
                    is_const: false,
                    is_async: false,
                });
            }
        }
    }

    methods
}

/// Parse a method element into RustMethodInfo
fn parse_method_element(element: &scraper::ElementRef) -> Option<RustMethodInfo> {
    // Try to find method signature
    let sig_selector = Selector::parse(".fn-signature, h4 code, .method-signature, code").ok()?;

    let signature = element
        .select(&sig_selector)
        .next()
        .map(|e| clean_text(&e.text().collect::<String>()))?;

    if signature.is_empty() {
        return None;
    }

    // Extract method name from signature
    let name = extract_method_name(&signature)?;

    // Get summary from docblock-short
    let summary_selector = Selector::parse(".docblock-short, .docblock p:first-child").ok()?;
    let summary = element
        .select(&summary_selector)
        .next()
        .map(|e| clean_text(&e.text().collect::<String>()))
        .unwrap_or_default();

    let is_unsafe = signature.contains("unsafe ");
    let is_const = signature.contains("const ");
    let is_async = signature.contains("async ");

    Some(RustMethodInfo {
        name,
        signature,
        summary,
        is_unsafe,
        is_const,
        is_async,
    })
}

/// Extract method name from a signature
fn extract_method_name(signature: &str) -> Option<String> {
    // Pattern: fn method_name(...) or pub fn method_name(...)
    let re = Regex::new(r"fn\s+(\w+)").ok()?;
    re.captures(signature)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extract implemented traits
fn extract_impl_traits(document: &Html) -> Vec<String> {
    let mut traits = Vec::new();

    let impl_selector = Selector::parse("#trait-implementations-list .impl, .impl-items h3").ok();

    if let Some(selector) = impl_selector {
        for element in document.select(&selector).take(30) {
            let text = clean_text(&element.text().collect::<String>());

            // Look for "impl Trait for Type" pattern
            if text.contains("impl ") {
                // Extract just the trait name
                let re = Regex::new(r"impl(?:<[^>]*>)?\s+(\w+(?:<[^>]*>)?)").ok();
                if let Some(captures) = re.and_then(|r| r.captures(&text)) {
                    if let Some(m) = captures.get(1) {
                        let trait_name = m.as_str().to_string();
                        if !traits.contains(&trait_name) {
                            traits.push(trait_name);
                        }
                    }
                }
            }
        }
    }

    traits
}

/// Extract associated types for traits
fn extract_associated_types(document: &Html) -> Vec<RustAssociatedType> {
    let mut types = Vec::new();

    let assoc_selector = Selector::parse(".associatedtype").ok();

    if let Some(selector) = assoc_selector {
        for element in document.select(&selector).take(20) {
            let text = clean_text(&element.text().collect::<String>());

            // Parse "type Name: Bounds = Default"
            let re = Regex::new(r"type\s+(\w+)(?:\s*:\s*([^=]+))?(?:\s*=\s*(.+))?").ok();
            if let Some(captures) = re.and_then(|r| r.captures(&text)) {
                let name = captures.get(1).map(|m| m.as_str().to_string());
                let bounds = captures.get(2).map(|m| m.as_str().trim().to_string());
                let default = captures.get(3).map(|m| m.as_str().trim().to_string());

                if let Some(name) = name {
                    types.push(RustAssociatedType {
                        name,
                        bounds,
                        default,
                    });
                }
            }
        }
    }

    types
}

/// Extract source code URL
fn extract_source_url(document: &Html) -> Option<String> {
    let source_selector = Selector::parse("a.src, .src-content a, a[href*='src/']").ok()?;

    document.select(&source_selector).next().and_then(|e| {
        e.value().attr("href").map(|href| {
            // Make absolute if relative
            if href.starts_with("http") {
                href.to_string()
            } else if href.starts_with('/') {
                format!("https://doc.rust-lang.org{}", href)
            } else {
                href.to_string()
            }
        })
    })
}

/// Clean and normalize text
fn clean_text(text: &str) -> String {
    // Normalize whitespace
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(text.trim(), " ").to_string()
}

/// Clean code blocks
fn clean_code(code: &str) -> String {
    // Remove line numbers if present, trim each line
    code.lines()
        .map(|line| {
            // Remove leading line numbers like "1 " or "  1 "
            let trimmed = line.trim_start();
            if trimmed.chars().take_while(|c| c.is_ascii_digit()).count() > 0 {
                let after_num = trimmed.trim_start_matches(|c: char| c.is_ascii_digit());
                if after_num.starts_with(' ') || after_num.starts_with('\t') {
                    return after_num.trim_start();
                }
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// Extract the title from an HTML page
pub fn extract_title_from_html(html: &str) -> Option<String> {
    let document = Html::parse_document(html);

    // Try to find the main heading
    let h1_selector = Selector::parse("h1.fqn, h1 .in-band, main h1").ok()?;
    if let Some(element) = document.select(&h1_selector).next() {
        let text = clean_text(&element.text().collect::<String>());
        if !text.is_empty() {
            // Remove common suffixes like "in std::collections" or "Struct"
            let cleaned = text
                .split(" in ")
                .next()
                .unwrap_or(&text)
                .trim();
            return Some(cleaned.to_string());
        }
    }

    // Fallback to title tag
    let title_selector = Selector::parse("title").ok()?;
    if let Some(element) = document.select(&title_selector).next() {
        let text = element.text().collect::<String>();
        // Title format is usually "ItemName - Rust" or "ItemName in crate - Rust"
        let cleaned = text
            .split(" - ")
            .next()
            .unwrap_or(&text)
            .split(" in ")
            .next()
            .unwrap_or(&text)
            .trim();
        if !cleaned.is_empty() {
            return Some(cleaned.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        assert_eq!(clean_text("  hello   world  "), "hello world");
        assert_eq!(clean_text("foo\n\nbar"), "foo bar");
    }

    #[test]
    fn test_extract_method_name() {
        assert_eq!(
            extract_method_name("pub fn insert(&mut self, k: K, v: V) -> Option<V>"),
            Some("insert".to_string())
        );
        assert_eq!(
            extract_method_name("fn new() -> Self"),
            Some("new".to_string())
        );
    }
}
