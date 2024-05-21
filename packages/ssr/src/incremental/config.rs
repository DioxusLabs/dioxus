#![allow(non_snake_case)]

use crate::incremental::IncrementalRenderer;
use crate::incremental::IncrementalRendererError;

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use super::fs_cache::PathMapFn;
use super::memory_cache::InMemoryCache;

/// Something that can render a HTML page from a body.
pub trait WrapBody {
    /// Render the HTML before the body
    fn render_before_body<R: Write>(&self, to: &mut R) -> Result<(), IncrementalRendererError>;
    /// Render the HTML after the body
    fn render_after_body<R: Write>(&self, to: &mut R) -> Result<(), IncrementalRendererError>;

    /// Wrap the body of the page in the wrapper.
    fn wrap_body(&self, body: &str) -> String {
        let mut bytes = Vec::new();
        self.render_before_body(&mut bytes).unwrap();
        bytes.extend_from_slice(body.as_bytes());
        self.render_after_body(&mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }
}

/// The default page renderer
pub struct DefaultRenderer {
    /// The HTML before the body.
    pub before_body: String,
    /// The HTML after the body.
    pub after_body: String,
}

impl Default for DefaultRenderer {
    fn default() -> Self {
        let title = dioxus_cli_config::CURRENT_CONFIG
            .as_ref()
            .map(|c| c.dioxus_config.application.name.clone())
            .unwrap_or("Dioxus Application".into());
        let before = format!(
            r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>{}</title>
        </head>
        <body>"#,
            title
        );
        let after = r#"</body>
        </html>"#;
        Self {
            before_body: before.to_string(),
            after_body: after.to_string(),
        }
    }
}

impl WrapBody for DefaultRenderer {
    fn render_before_body<R: Write>(&self, to: &mut R) -> Result<(), IncrementalRendererError> {
        to.write_all(self.before_body.as_bytes())?;
        Ok(())
    }

    fn render_after_body<R: Write>(&self, to: &mut R) -> Result<(), IncrementalRendererError> {
        to.write_all(self.after_body.as_bytes())?;
        Ok(())
    }
}

/// A configuration for the incremental renderer.
#[derive(Clone)]
pub struct IncrementalRendererConfig {
    static_dir: PathBuf,
    memory_cache_limit: usize,
    invalidate_after: Option<Duration>,
    map_path: Option<PathMapFn>,
    clear_cache: bool,
}

impl Default for IncrementalRendererConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl IncrementalRendererConfig {
    /// Create a new incremental renderer configuration.
    pub fn new() -> Self {
        Self {
            static_dir: PathBuf::from("./static"),
            memory_cache_limit: 10000,
            invalidate_after: None,
            map_path: None,
            clear_cache: true,
        }
    }

    /// Clear the cache on startup (default: true)
    pub fn clear_cache(mut self, clear_cache: bool) -> Self {
        self.clear_cache = clear_cache;
        self
    }

    /// Set a mapping from the route to the file path. This will override the default mapping configured with `static_dir`.
    /// The function should return the path to the folder to store the index.html file in.
    pub fn map_path<F: Fn(&str) -> PathBuf + Send + Sync + 'static>(mut self, map_path: F) -> Self {
        self.map_path = Some(Arc::new(map_path));
        self
    }

    /// Set the static directory.
    pub fn static_dir<P: AsRef<Path>>(mut self, static_dir: P) -> Self {
        self.static_dir = static_dir.as_ref().to_path_buf();
        self
    }

    /// Set the memory cache limit.
    pub const fn memory_cache_limit(mut self, memory_cache_limit: usize) -> Self {
        self.memory_cache_limit = memory_cache_limit;
        self
    }

    /// Set the invalidation time.
    pub fn invalidate_after(mut self, invalidate_after: Duration) -> Self {
        self.invalidate_after = Some(invalidate_after);
        self
    }

    /// Build the incremental renderer.
    pub fn build(self) -> IncrementalRenderer {
        let mut renderer = IncrementalRenderer {
            #[cfg(not(target_arch = "wasm32"))]
            file_system_cache: crate::incremental::fs_cache::FileSystemCache::new(
                self.static_dir.clone(),
                self.map_path,
                self.invalidate_after,
            ),
            memory_cache: InMemoryCache::new(self.memory_cache_limit, self.invalidate_after),
            invalidate_after: self.invalidate_after,
        };

        if self.clear_cache {
            renderer.invalidate_all();
        }

        renderer
    }
}
