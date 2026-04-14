#![allow(non_snake_case)]

#[cfg(not(target_arch = "wasm32"))]
use crate::isrg::fs_cache::PathMapFn;

use crate::isrg::memory_cache::InMemoryCache;
use crate::IncrementalRenderer;

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

/// A configuration for the incremental renderer.
#[derive(Clone)]
pub struct IncrementalRendererConfig {
    static_dir: PathBuf,
    memory_cache_limit: usize,
    invalidate_after: Option<Duration>,
    clear_cache: bool,
    pre_render: bool,

    #[cfg(not(target_arch = "wasm32"))]
    map_path: Option<PathMapFn>,
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
            clear_cache: false,
            pre_render: false,
            #[cfg(not(target_arch = "wasm32"))]
            map_path: None,
        }
    }

    /// Clear the cache on startup (default: true)
    pub fn clear_cache(mut self, clear_cache: bool) -> Self {
        self.clear_cache = clear_cache;
        self
    }

    /// Set a mapping from the route to the file path. This will override the default mapping configured with `static_dir`.
    /// The function should return the path to the folder to store the index.html file in.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn map_path<F: Fn(&str) -> PathBuf + Send + Sync + 'static>(mut self, map_path: F) -> Self {
        self.map_path = Some(std::sync::Arc::new(map_path));
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

    /// Set whether to include hydration ids in the pre-rendered html.
    pub fn pre_render(mut self, pre_render: bool) -> Self {
        self.pre_render = pre_render;
        self
    }

    /// Build the incremental renderer.
    pub fn build(self) -> IncrementalRenderer {
        let mut renderer = IncrementalRenderer {
            #[cfg(not(target_arch = "wasm32"))]
            file_system_cache: super::fs_cache::FileSystemCache::new(
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
