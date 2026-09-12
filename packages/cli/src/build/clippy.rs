//! Generation of a merged `clippy.toml` for `dx clippy` builds.
//!
//! Clippy reads its config from `clippy.toml` / `.clippy.toml` discovered by walking up from the
//! crate dir. We want to layer Dioxus-aware lints (eg guards held across await points, std APIs
//! that don't exist on wasm) on top of the user's own config, so we generate a merged config into
//! the session cache dir and point clippy at it with `CLIPPY_CONF_DIR`.

use super::BuildRequest;
use crate::{BundleFormat, Result};
use std::path::PathBuf;

impl BuildRequest {
    /// Write the merged clippy config (user's `clippy.toml` plus generated Dioxus entries) into the
    /// session cache dir and return the dir containing it, suitable for `CLIPPY_CONF_DIR`.
    pub(crate) fn write_clippy_config(&self) -> Result<PathBuf> {
        let merged = merge_clippy_config(
            self.find_user_clippy_config().unwrap_or_default(),
            generated_clippy_config(self.bundle),
        );

        let dir = self.session_cache_dir().join("clippy");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("clippy.toml"), toml::to_string(&merged)?)?;
        Ok(dir)
    }

    /// Find the user's clippy config, searching from the package manifest dir upward to the
    /// workspace root (clippy's own lookup order: `.clippy.toml` then `clippy.toml` in each dir).
    fn find_user_clippy_config(&self) -> Option<toml::Table> {
        let workspace_dir = self.workspace_dir();
        let mut dir = Some(self.package_manifest_dir());

        while let Some(current) = dir {
            for name in [".clippy.toml", "clippy.toml"] {
                if let Ok(contents) = std::fs::read_to_string(current.join(name)) {
                    return contents.parse::<toml::Table>().ok();
                }
            }

            if current == workspace_dir {
                break;
            }
            dir = current.parent().map(|p| p.to_path_buf());
        }

        None
    }
}

/// Merge generated entries into the user's clippy config.
///
/// For each generated key: if both values are arrays, generated items that aren't already present
/// are appended; if the user set a non-array value, the user's value wins; if absent, insert.
fn merge_clippy_config(mut user: toml::Table, generated: toml::Table) -> toml::Table {
    for (key, generated_value) in generated {
        match (user.get_mut(&key), generated_value) {
            (Some(toml::Value::Array(user_items)), toml::Value::Array(generated_items)) => {
                for item in generated_items {
                    if !user_items.contains(&item) {
                        user_items.push(item);
                    }
                }
            }
            (Some(_), _) => {}
            (None, value) => {
                user.insert(key, value);
            }
        }
    }
    user
}

/// The Dioxus-aware clippy config entries for a given bundle format.
fn generated_clippy_config(bundle: BundleFormat) -> toml::Table {
    fn entry(path: &str, reason: &str) -> toml::Value {
        let mut table = toml::Table::new();
        table.insert("path".to_string(), path.into());
        table.insert("reason".to_string(), reason.into());
        toml::Value::Table(table)
    }

    fn array(entries: Vec<toml::Value>) -> toml::Value {
        toml::Value::Array(entries)
    }

    let mut config = toml::Table::new();

    config.insert(
        "await-holding-invalid-types".to_string(),
        array(vec![
            entry(
                "dioxus_signals::WriteLock",
                "holding a signal write guard across an await point will panic or deadlock other readers",
            ),
            entry(
                "generational_box::GenerationalRefMut",
                "holding a signal write guard across an await point will panic or deadlock other readers",
            ),
            entry(
                "generational_box::GenerationalRef",
                "holding a signal read guard across an await point blocks writers",
            ),
        ]),
    );

    match bundle {
        BundleFormat::Web => {
            const FS_REASON: &str = "the filesystem is unavailable on wasm; use asset!() or fetch";
            config.insert(
                "disallowed-methods".to_string(),
                array(vec![
                    entry(
                        "std::thread::spawn",
                        "threads are unavailable on wasm32-unknown-unknown; use dioxus::prelude::spawn or wasm_bindgen_futures::spawn_local",
                    ),
                    entry(
                        "std::thread::sleep",
                        "blocking sleep hangs the browser main thread; use an async timer",
                    ),
                    entry("std::fs::read", FS_REASON),
                    entry("std::fs::read_to_string", FS_REASON),
                    entry("std::fs::write", FS_REASON),
                    entry("std::fs::File::open", FS_REASON),
                    entry("std::fs::File::create", FS_REASON),
                    entry(
                        "std::net::TcpStream::connect",
                        "sockets are unavailable on wasm; use fetch/WebSocket APIs",
                    ),
                ]),
            );
            config.insert(
                "disallowed-types".to_string(),
                array(vec![entry(
                    "std::time::Instant",
                    "std::time::Instant panics on wasm; use web_time::Instant",
                )]),
            );
        }
        BundleFormat::Server => {
            const WEB_SYS_REASON: &str = "web_sys is client-only; gate with cfg(feature = \"web\")";
            config.insert(
                "disallowed-types".to_string(),
                array(vec![
                    entry("web_sys::Window", WEB_SYS_REASON),
                    entry("web_sys::Document", WEB_SYS_REASON),
                ]),
            );
        }
        _ => {}
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(table: &toml::Table, key: &str) -> Vec<String> {
        table[key]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.get("path").and_then(|p| p.as_str().map(String::from)))
            .collect()
    }

    #[test]
    fn generated_web_config() {
        let merged = merge_clippy_config(
            toml::Table::new(),
            generated_clippy_config(BundleFormat::Web),
        );
        assert!(merged.contains_key("await-holding-invalid-types"));
        assert!(paths(&merged, "disallowed-methods").contains(&"std::thread::spawn".to_string()));
    }

    #[test]
    fn merge_appends_without_duplicates() {
        let user: toml::Table = r#"disallowed-methods = [{ path = "foo::bar" }]"#.parse().unwrap();
        let merged = merge_clippy_config(user, generated_clippy_config(BundleFormat::Web));
        let methods = paths(&merged, "disallowed-methods");
        assert!(methods.contains(&"foo::bar".to_string()));
        assert!(methods.contains(&"std::thread::spawn".to_string()));

        // Merging twice shouldn't duplicate entries
        let merged = merge_clippy_config(merged, generated_clippy_config(BundleFormat::Web));
        assert_eq!(
            paths(&merged, "disallowed-methods")
                .iter()
                .filter(|p| *p == "std::thread::spawn")
                .count(),
            1
        );
    }

    #[test]
    fn merge_preserves_user_values() {
        let user: toml::Table = r#"
            msrv = "1.80"
            disallowed-methods = "user-value"
        "#
        .parse()
        .unwrap();
        let merged = merge_clippy_config(user, generated_clippy_config(BundleFormat::Web));
        assert_eq!(merged["msrv"].as_str(), Some("1.80"));
        // A non-array user value for a generated key is left alone
        assert_eq!(merged["disallowed-methods"].as_str(), Some("user-value"));
    }
}
