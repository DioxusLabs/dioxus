#![allow(non_snake_case)]

use crate::incremental::IncrementalRenderer;
use crate::incremental::IncrementalRendererError;

use std::{
    io::Write,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// Something that can render a HTML page from a body.
pub trait WrapBody {
    /// Render the HTML before the body
    fn render_before_body<R: Write>(&self, to: &mut R) -> Result<(), IncrementalRendererError>;
    /// Render the HTML after the body
    fn render_after_body<R: Write>(&self, to: &mut R) -> Result<(), IncrementalRendererError>;
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
        let before = r#"<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Dioxus Application</title>
        </head>
        <body>"#;
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

pub(crate) type PathMapFn = Arc<dyn Fn(&str) -> PathBuf + Send + Sync>;

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
        let static_dir = self.static_dir.clone();
        let mut renderer = IncrementalRenderer {
            static_dir: self.static_dir.clone(),
            memory_cache: NonZeroUsize::new(self.memory_cache_limit)
                .map(|limit| lru::LruCache::with_hasher(limit, Default::default())),
            invalidate_after: self.invalidate_after,
            ssr_renderer: crate::Renderer::new(),
            map_path: self.map_path.unwrap_or_else(move || {
                Arc::new(move |route: &str| {
                    let mut path = static_dir.clone();
                    for segment in route.split('/') {
                        path.push(segment);
                    }
                    path
                })
            }),
        };

        if self.clear_cache {
            renderer.invalidate_all();
        }

        renderer
    }
}
