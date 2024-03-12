//! Incremental file based incremental rendering

#![allow(non_snake_case)]

use crate::fs_cache::ValidCachedPath;
use chrono::offset::Utc;
use chrono::DateTime;
use dioxus_core::VirtualDom;
use rustc_hash::FxHasher;
use std::{
    future::Future,
    hash::BuildHasherDefault,
    ops::{Deref, DerefMut},
    path::PathBuf,
    pin::Pin,
    time::{Duration, SystemTime},
};
use tokio::io::{AsyncWrite, AsyncWriteExt};

pub use crate::fs_cache::*;
pub use crate::incremental_cfg::*;

/// An incremental renderer.
pub struct IncrementalRenderer {
    pub(crate) static_dir: PathBuf,
    #[allow(clippy::type_complexity)]
    pub(crate) memory_cache:
        Option<lru::LruCache<String, (DateTime<Utc>, Vec<u8>), BuildHasherDefault<FxHasher>>>,
    pub(crate) invalidate_after: Option<Duration>,
    pub(crate) ssr_renderer: crate::Renderer,
    pub(crate) map_path: PathMapFn,
}

impl IncrementalRenderer {
    /// Get the inner renderer.
    pub fn renderer(&self) -> &crate::Renderer {
        &self.ssr_renderer
    }

    /// Get the inner renderer mutably.
    pub fn renderer_mut(&mut self) -> &mut crate::Renderer {
        &mut self.ssr_renderer
    }

    /// Create a new incremental renderer builder.
    pub fn builder() -> IncrementalRendererConfig {
        IncrementalRendererConfig::new()
    }

    /// Remove a route from the cache.
    pub fn invalidate(&mut self, route: &str) {
        if let Some(cache) = &mut self.memory_cache {
            cache.pop(route);
        }
        if let Some(path) = self.find_file(route) {
            let _ = std::fs::remove_file(path.full_path);
        }
    }

    /// Remove all routes from the cache.
    pub fn invalidate_all(&mut self) {
        if let Some(cache) = &mut self.memory_cache {
            cache.clear();
        }
        // clear the static directory
        let _ = std::fs::remove_dir_all(&self.static_dir);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn track_timestamps(&self) -> bool {
        self.invalidate_after.is_some()
    }

    async fn render_and_cache<'a, R: WrapBody + Send + Sync>(
        &'a mut self,
        route: String,
        mut virtual_dom: VirtualDom,
        output: &'a mut (impl AsyncWrite + Unpin + Send),
        rebuild_with: impl FnOnce(&mut VirtualDom) -> Pin<Box<dyn Future<Output = ()> + '_>>,
        renderer: &'a R,
    ) -> Result<RenderFreshness, IncrementalRendererError> {
        let mut html_buffer = WriteBuffer { buffer: Vec::new() };
        {
            rebuild_with(&mut virtual_dom).await;

            renderer.render_before_body(&mut *html_buffer)?;
            self.ssr_renderer
                .render_to(&mut html_buffer, &virtual_dom)?;
        }
        renderer.render_after_body(&mut *html_buffer)?;
        let html_buffer = html_buffer.buffer;

        output.write_all(&html_buffer).await?;

        self.add_to_cache(route, html_buffer)
    }

    fn add_to_cache(
        &mut self,
        route: String,
        html: Vec<u8>,
    ) -> Result<RenderFreshness, IncrementalRendererError> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::io::Write;
            let file_path = self.route_as_path(&route);
            if let Some(parent) = file_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            let file = std::fs::File::create(file_path)?;
            let mut file = std::io::BufWriter::new(file);
            file.write_all(&html)?;
        }
        self.add_to_memory_cache(route, html);
        Ok(RenderFreshness::now(self.invalidate_after))
    }

    fn add_to_memory_cache(&mut self, route: String, html: Vec<u8>) {
        if let Some(cache) = self.memory_cache.as_mut() {
            cache.put(route, (Utc::now(), html));
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn promote_memory_cache<K: AsRef<str>>(&mut self, route: K) {
        if let Some(cache) = self.memory_cache.as_mut() {
            cache.promote(route.as_ref())
        }
    }

    async fn search_cache(
        &mut self,
        route: String,
        output: &mut (impl AsyncWrite + Unpin + std::marker::Send),
    ) -> Result<Option<RenderFreshness>, IncrementalRendererError> {
        // check the memory cache
        if let Some((timestamp, cache_hit)) = self
            .memory_cache
            .as_mut()
            .and_then(|cache| cache.get(&route))
        {
            let now = Utc::now();
            let elapsed = timestamp.signed_duration_since(now);
            let age = elapsed.num_seconds();
            if let Some(invalidate_after) = self.invalidate_after {
                if elapsed.to_std().unwrap() < invalidate_after {
                    tracing::trace!("memory cache hit {:?}", route);
                    output.write_all(cache_hit).await?;
                    let max_age = invalidate_after.as_secs();
                    return Ok(Some(RenderFreshness::new(age as u64, max_age)));
                }
            } else {
                tracing::trace!("memory cache hit {:?}", route);
                output.write_all(cache_hit).await?;
                return Ok(Some(RenderFreshness::new_age(age as u64)));
            }
        }
        // check the file cache
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(file_path) = self.find_file(&route) {
            if let Some(freshness) = file_path.freshness(self.invalidate_after) {
                if let Ok(file) = tokio::fs::File::open(file_path.full_path).await {
                    let mut file = tokio::io::BufReader::new(file);
                    tokio::io::copy_buf(&mut file, output).await?;
                    tracing::trace!("file cache hit {:?}", route);
                    self.promote_memory_cache(&route);
                    return Ok(Some(freshness));
                }
            }
        }
        Ok(None)
    }

    /// Render a route or get it from cache.
    pub async fn render<R: WrapBody + Send + Sync>(
        &mut self,
        route: String,
        virtual_dom_factory: impl FnOnce() -> VirtualDom,
        output: &mut (impl AsyncWrite + Unpin + std::marker::Send),
        rebuild_with: impl FnOnce(&mut VirtualDom) -> Pin<Box<dyn Future<Output = ()> + '_>>,
        renderer: &R,
    ) -> Result<RenderFreshness, IncrementalRendererError> {
        // check if this route is cached
        if let Some(freshness) = self.search_cache(route.to_string(), output).await? {
            Ok(freshness)
        } else {
            // if not, create it
            let freshness = self
                .render_and_cache(route, virtual_dom_factory(), output, rebuild_with, renderer)
                .await?;
            tracing::trace!("cache miss");
            Ok(freshness)
        }
    }

    fn find_file(&self, route: &str) -> Option<ValidCachedPath> {
        let mut file_path = (self.map_path)(route);
        if let Some(deadline) = self.invalidate_after {
            // find the first file that matches the route and is a html file
            file_path.push("index");
            if let Ok(dir) = std::fs::read_dir(file_path) {
                let mut file = None;
                for entry in dir.flatten() {
                    if let Some(cached_path) = ValidCachedPath::try_from_path(entry.path()) {
                        if let Ok(elapsed) = cached_path.timestamp.elapsed() {
                            if elapsed < deadline {
                                file = Some(cached_path);
                                continue;
                            }
                        }
                        // if the timestamp is invalid or passed, delete the file
                        if let Err(err) = std::fs::remove_file(entry.path()) {
                            tracing::error!("Failed to remove file: {}", err);
                        }
                    }
                }
                file
            } else {
                None
            }
        } else {
            file_path.push("index.html");
            file_path.exists().then_some({
                ValidCachedPath {
                    full_path: file_path,
                    timestamp: SystemTime::now(),
                }
            })
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn route_as_path(&self, route: &str) -> PathBuf {
        let mut file_path = (self.map_path)(route);
        if self.track_timestamps() {
            file_path.push("index");
            file_path.push(timestamp());
        } else {
            file_path.push("index");
        }
        file_path.set_extension("html");
        file_path
    }
}

struct WriteBuffer {
    buffer: Vec<u8>,
}

impl std::fmt::Write for WriteBuffer {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.buffer.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

impl Deref for WriteBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for WriteBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// An error that can occur while rendering a route or retrieving a cached route.
#[derive(Debug, thiserror::Error)]
pub enum IncrementalRendererError {
    /// An formatting error occurred while rendering a route.
    #[error("RenderError: {0}")]
    RenderError(#[from] std::fmt::Error),
    /// An IO error occurred while rendering a route.
    #[error("IoError: {0}")]
    IoError(#[from] std::io::Error),
    /// An IO error occurred while rendering a route.
    #[error("Other: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
