//! Extract and merge inline Dioxus.toml configuration from Rust source files.
//!
//! This module allows examples and tests to embed configuration in their doc comments:
//!
//! ```rust
//! #![allow(unused)]
//! //! This example demonstrates iOS geolocation
//! //!
//! //! ```dioxus.toml
//! //! [bundle]
//! //! identifier = "com.example.geolocation"
//! //!
//! //! [permissions]
//! //! location = { precision = "fine", description = "Track location" }
//! //! ```
//! ```
//!
//! The embedded config is deep-merged with the workspace/app Dioxus.toml,
//! with inline values taking precedence.

use std::path::Path;

/// Extract embedded Dioxus.toml configuration from a Rust source file.
///
/// Parses the file's module-level doc comments (`//!`) looking for a fenced
/// code block marked as `dioxus.toml`. Returns the parsed TOML as a generic
/// Value for merging.
///
/// Handles inner attributes (`#![...]`) that may appear before doc comments.
///
/// # Example
///
/// ```rust,ignore
/// let source = r#"
/// #![allow(unused)]
/// //! Example showing permissions
/// //!
/// //! ```dioxus.toml
/// //! [permissions]
/// //! camera = { description = "Take photos" }
/// //! ```
///
/// fn main() {}
/// "#;
///
/// let config = extract_inline_config(source).unwrap();
/// ```
pub fn extract_inline_config(source: &str) -> Option<toml::Value> {
    let doc_content = extract_module_doc_comments(source)?;
    let toml_content = extract_toml_block(&doc_content)?;
    toml::from_str(&toml_content).ok()
}

/// Extract inline config from a file path.
pub fn extract_inline_config_from_file(path: &Path) -> Option<toml::Value> {
    let source = std::fs::read_to_string(path).ok()?;
    extract_inline_config(&source)
}

/// Extract module-level doc comments (`//!`) from the beginning of a source file.
///
/// Handles:
/// - Inner attributes (`#![...]`) that may precede doc comments
/// - Empty lines between attributes and doc comments
/// - Multi-line inner attributes
fn extract_module_doc_comments(source: &str) -> Option<String> {
    let mut doc_lines = Vec::new();
    let mut in_doc_section = false;
    let mut in_multiline_attr = false;

    for line in source.lines() {
        let trimmed = line.trim();

        // Handle multi-line inner attributes (tracking bracket depth would be
        // more robust, but this handles the common case)
        if in_multiline_attr {
            if trimmed.ends_with(']') || trimmed.contains(")]") {
                in_multiline_attr = false;
            }
            continue;
        }

        // Skip empty lines before we've started collecting doc comments
        if !in_doc_section && trimmed.is_empty() {
            continue;
        }

        // Skip inner attributes (#![...])
        if trimmed.starts_with("#![") {
            // Check if it's a multi-line attribute (doesn't close on same line)
            if !trimmed.ends_with(']') {
                in_multiline_attr = true;
            }
            continue;
        }

        // Also handle inner attributes with the #! on one line and [ on another (rare but possible)
        // and block-style attributes like #![cfg(...)]
        if trimmed.starts_with("#!") {
            continue;
        }

        // Check for module doc comment
        if let Some(content) = trimmed.strip_prefix("//!") {
            in_doc_section = true;
            // Remove the leading space if present
            let content = content.strip_prefix(' ').unwrap_or(content);
            doc_lines.push(content.to_string());
        } else if in_doc_section {
            // We've hit a non-doc-comment line after starting, stop collecting
            break;
        } else if !trimmed.is_empty() {
            // Hit actual code before any doc comments
            break;
        }
    }

    if doc_lines.is_empty() {
        None
    } else {
        Some(doc_lines.join("\n"))
    }
}

/// Extract the content of a ```dioxus.toml fenced code block.
fn extract_toml_block(doc_content: &str) -> Option<String> {
    let mut in_toml_block = false;
    let mut toml_lines = Vec::new();

    for line in doc_content.lines() {
        if !in_toml_block {
            // Look for opening fence with dioxus.toml marker
            let trimmed = line.trim();
            if trimmed == "```dioxus.toml" || trimmed == "```toml dioxus.toml" {
                in_toml_block = true;
                continue;
            }
        } else {
            // Check for closing fence
            if line.trim() == "```" {
                break;
            }
            toml_lines.push(line);
        }
    }

    if toml_lines.is_empty() {
        None
    } else {
        Some(toml_lines.join("\n"))
    }
}

/// Deep merge two TOML values, with `override_val` taking precedence.
///
/// - For tables: recursively merge, with override values winning
/// - For arrays: override replaces base entirely
/// - For other values: override replaces base
pub fn merge_toml_values(base: toml::Value, override_val: toml::Value) -> toml::Value {
    match (base, override_val) {
        // Both are tables - deep merge
        (toml::Value::Table(mut base_table), toml::Value::Table(override_table)) => {
            for (key, override_value) in override_table {
                match base_table.remove(&key) {
                    Some(base_value) => {
                        // Recursively merge
                        base_table.insert(key, merge_toml_values(base_value, override_value));
                    }
                    None => {
                        // Key only in override, just insert
                        base_table.insert(key, override_value);
                    }
                }
            }
            toml::Value::Table(base_table)
        }
        // Override is not a table, or base is not a table - override wins
        (_, override_val) => override_val,
    }
}

/// Merge an inline config with a base DioxusConfig.
///
/// The inline config (from doc comments) takes precedence over the base config.
pub fn merge_with_inline_config(
    base: &super::DioxusConfig,
    inline: toml::Value,
) -> Result<super::DioxusConfig, toml::de::Error> {
    // Convert base to toml::Value
    let base_str = toml::to_string(base).expect("DioxusConfig should serialize");
    let base_value: toml::Value = toml::from_str(&base_str).expect("Should parse back");

    // Merge
    let merged = merge_toml_values(base_value, inline);

    // Convert back to DioxusConfig
    merged.try_into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_module_doc_comments() {
        let source = r#"//! This is an example
//! with multiple lines
//!
//! And a blank line

fn main() {}
"#;

        let docs = extract_module_doc_comments(source).unwrap();
        assert!(docs.contains("This is an example"));
        assert!(docs.contains("with multiple lines"));
        assert!(docs.contains("And a blank line"));
    }

    #[test]
    fn test_extract_doc_comments_with_inner_attrs() {
        let source = r#"#![allow(unused)]
#![cfg(feature = "something")]
//! This is an example
//! with inner attributes before it

fn main() {}
"#;

        let docs = extract_module_doc_comments(source).unwrap();
        assert!(docs.contains("This is an example"));
        assert!(docs.contains("with inner attributes before it"));
        assert!(!docs.contains("allow"));
        assert!(!docs.contains("cfg"));
    }

    #[test]
    fn test_extract_doc_comments_with_multiline_attr() {
        let source = r#"#![cfg(
    all(
        feature = "something",
        target_os = "ios"
    )
)]
//! This is an example

fn main() {}
"#;

        let docs = extract_module_doc_comments(source).unwrap();
        assert!(docs.contains("This is an example"));
        assert!(!docs.contains("cfg"));
        assert!(!docs.contains("feature"));
    }

    #[test]
    fn test_extract_toml_block() {
        let doc = r#"This is an example

```dioxus.toml
[bundle]
identifier = "com.test"
```

More text here"#;

        let toml = extract_toml_block(doc).unwrap();
        assert!(toml.contains("[bundle]"));
        assert!(toml.contains("identifier"));
    }

    #[test]
    fn test_extract_inline_config() {
        let source = r#"//! Example app
//!
//! ```dioxus.toml
//! [bundle]
//! identifier = "com.example.test"
//!
//! [permissions]
//! camera = { description = "Take photos" }
//! ```

fn main() {}
"#;

        let config = extract_inline_config(source).unwrap();
        let table = config.as_table().unwrap();

        assert!(table.contains_key("bundle"));
        assert!(table.contains_key("permissions"));

        let bundle = table["bundle"].as_table().unwrap();
        assert_eq!(bundle["identifier"].as_str().unwrap(), "com.example.test");
    }

    #[test]
    fn test_extract_inline_config_with_attrs() {
        let source = r#"#![allow(unused)]
#![cfg(target_os = "ios")]
//! iOS Example
//!
//! ```dioxus.toml
//! [bundle]
//! identifier = "com.example.ios"
//! ```

fn main() {}
"#;

        let config = extract_inline_config(source).unwrap();
        let table = config.as_table().unwrap();
        let bundle = table["bundle"].as_table().unwrap();
        assert_eq!(bundle["identifier"].as_str().unwrap(), "com.example.ios");
    }

    #[test]
    fn test_no_inline_config() {
        let source = r#"//! Just a regular example
//! No config here

fn main() {}
"#;

        assert!(extract_inline_config(source).is_none());
    }

    #[test]
    fn test_merge_toml_values_simple() {
        let base: toml::Value = toml::from_str(
            r#"
            [bundle]
            identifier = "com.base"
            publisher = "Base Publisher"
            "#,
        )
        .unwrap();

        let override_val: toml::Value = toml::from_str(
            r#"
            [bundle]
            identifier = "com.override"
            "#,
        )
        .unwrap();

        let merged = merge_toml_values(base, override_val);
        let table = merged.as_table().unwrap();
        let bundle = table["bundle"].as_table().unwrap();

        // Override wins for identifier
        assert_eq!(bundle["identifier"].as_str().unwrap(), "com.override");
        // Base value preserved for publisher
        assert_eq!(bundle["publisher"].as_str().unwrap(), "Base Publisher");
    }

    #[test]
    fn test_merge_toml_values_deep() {
        let base: toml::Value = toml::from_str(
            r#"
            [permissions]
            camera = { description = "Base camera" }
            location = { precision = "coarse", description = "Base location" }
            "#,
        )
        .unwrap();

        let override_val: toml::Value = toml::from_str(
            r#"
            [permissions]
            location = { precision = "fine" }
            microphone = { description = "Record audio" }
            "#,
        )
        .unwrap();

        let merged = merge_toml_values(base, override_val);
        let perms = merged["permissions"].as_table().unwrap();

        // camera untouched
        assert_eq!(
            perms["camera"]["description"].as_str().unwrap(),
            "Base camera"
        );

        // location: precision overridden, description preserved
        assert_eq!(perms["location"]["precision"].as_str().unwrap(), "fine");
        assert_eq!(
            perms["location"]["description"].as_str().unwrap(),
            "Base location"
        );

        // microphone added
        assert_eq!(
            perms["microphone"]["description"].as_str().unwrap(),
            "Record audio"
        );
    }

    #[test]
    fn test_merge_adds_new_sections() {
        let base: toml::Value = toml::from_str(
            r#"
            [bundle]
            identifier = "com.base"
            "#,
        )
        .unwrap();

        let override_val: toml::Value = toml::from_str(
            r#"
            [permissions]
            camera = { description = "Photos" }
            "#,
        )
        .unwrap();

        let merged = merge_toml_values(base, override_val);
        let table = merged.as_table().unwrap();

        // Both sections present
        assert!(table.contains_key("bundle"));
        assert!(table.contains_key("permissions"));
    }
}
