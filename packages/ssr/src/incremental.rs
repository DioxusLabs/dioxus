//! Incremental file based incremental rendering

#![allow(non_snake_case)]

use dioxus_core::{Element, Scope, VirtualDom};
use rustc_hash::FxHasher;
use std::{
    hash::BuildHasherDefault,
    io::Write,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufReader};

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

/// A configuration for the incremental renderer.
#[derive(Clone)]
pub struct IncrementalRendererConfig {
    static_dir: PathBuf,
    memory_cache_limit: usize,
    invalidate_after: Option<Duration>,
    map_path: Option<Arc<dyn Fn(&str) -> PathBuf + Send + Sync>>,
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
        }
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
        IncrementalRenderer {
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
        }
    }
}

/// An incremental renderer.
pub struct IncrementalRenderer {
    static_dir: PathBuf,
    #[allow(clippy::type_complexity)]
    memory_cache:
        Option<lru::LruCache<String, (SystemTime, Vec<u8>), BuildHasherDefault<FxHasher>>>,
    invalidate_after: Option<Duration>,
    ssr_renderer: crate::Renderer,
    map_path: Arc<dyn Fn(&str) -> PathBuf + Send + Sync>,
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

    fn track_timestamps(&self) -> bool {
        self.invalidate_after.is_some()
    }

    fn render_and_cache<'a, P: 'static, R: WrapBody + Send + Sync>(
        &'a mut self,
        route: String,
        comp: fn(Scope<P>) -> Element,
        props: P,
        output: &'a mut (impl AsyncWrite + Unpin + Send),
        rebuild_with: impl FnOnce(&mut VirtualDom),
        renderer: &'a R,
    ) -> impl std::future::Future<Output = Result<RenderFreshness, IncrementalRendererError>> + 'a + Send
    {
        let mut html_buffer = WriteBuffer { buffer: Vec::new() };
        let result_1;
        let result2;
        {
            let mut vdom = VirtualDom::new_with_props(comp, props);
            rebuild_with(&mut vdom);

            result_1 = renderer.render_before_body(&mut *html_buffer);
            result2 = self.ssr_renderer.render_to(&mut html_buffer, &vdom);
        }
        async move {
            result_1?;
            result2?;
            renderer.render_after_body(&mut *html_buffer)?;
            let html_buffer = html_buffer.buffer;

            output.write_all(&html_buffer).await?;

            self.add_to_cache(route, html_buffer)
        }
    }

    fn add_to_cache(
        &mut self,
        route: String,
        html: Vec<u8>,
    ) -> Result<RenderFreshness, IncrementalRendererError> {
        let file_path = self.route_as_path(&route);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = std::fs::File::create(file_path)?;
        let mut file = std::io::BufWriter::new(file);
        file.write_all(&html)?;
        self.add_to_memory_cache(route, html);
        Ok(RenderFreshness::now(self.invalidate_after))
    }

    fn add_to_memory_cache(&mut self, route: String, html: Vec<u8>) {
        if let Some(cache) = self.memory_cache.as_mut() {
            cache.put(route, (SystemTime::now(), html));
        }
    }

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
            if let Ok(elapsed) = timestamp.elapsed() {
                let age = elapsed.as_secs();
                if let Some(invalidate_after) = self.invalidate_after {
                    if elapsed < invalidate_after {
                        log::trace!("memory cache hit {:?}", route);
                        output.write_all(cache_hit).await?;
                        let max_age = invalidate_after.as_secs();
                        return Ok(Some(RenderFreshness::new(age, max_age)));
                    }
                } else {
                    log::trace!("memory cache hit {:?}", route);
                    output.write_all(cache_hit).await?;
                    return Ok(Some(RenderFreshness::new_age(age)));
                }
            }
        }
        // check the file cache
        if let Some(file_path) = self.find_file(&route) {
            if let Some(freshness) = file_path.freshness(self.invalidate_after) {
                if let Ok(file) = tokio::fs::File::open(file_path.full_path).await {
                    let mut file = BufReader::new(file);
                    tokio::io::copy_buf(&mut file, output).await?;
                    log::trace!("file cache hit {:?}", route);
                    self.promote_memory_cache(&route);
                    return Ok(Some(freshness));
                }
            }
        }
        Ok(None)
    }

    /// Render a route or get it from cache.
    pub async fn render<P: 'static, R: WrapBody + Send + Sync>(
        &mut self,
        route: String,
        component: fn(Scope<P>) -> Element,
        props: P,
        output: &mut (impl AsyncWrite + Unpin + std::marker::Send),
        rebuild_with: impl FnOnce(&mut VirtualDom),
        renderer: &R,
    ) -> Result<RenderFreshness, IncrementalRendererError> {
        // check if this route is cached
        if let Some(freshness) = self.search_cache(route.to_string(), output).await? {
            Ok(freshness)
        } else {
            // if not, create it
            let freshness = self
                .render_and_cache(route, component, props, output, rebuild_with, renderer)
                .await?;
            log::trace!("cache miss");
            Ok(freshness)
        }
    }

    fn find_file(&self, route: &str) -> Option<ValidCachedPath> {
        let mut file_path = self.static_dir.clone();
        for segment in route.split('/') {
            file_path.push(segment);
        }
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
                            log::error!("Failed to remove file: {}", err);
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

/// Information about the freshness of a rendered response
#[derive(Debug, Clone, Copy)]
pub struct RenderFreshness {
    /// The age of the rendered response
    age: u64,
    /// The maximum age of the rendered response
    max_age: Option<u64>,
}

impl RenderFreshness {
    /// Create new freshness information
    pub fn new(age: u64, max_age: u64) -> Self {
        Self {
            age,
            max_age: Some(max_age),
        }
    }

    /// Create new freshness information with only the age
    pub fn new_age(age: u64) -> Self {
        Self { age, max_age: None }
    }

    /// Create new freshness information at the current time
    pub fn now(max_age: Option<Duration>) -> Self {
        Self {
            age: 0,
            max_age: max_age.map(|d| d.as_secs()),
        }
    }

    /// Get the age of the rendered response in seconds
    pub fn age(&self) -> u64 {
        self.age
    }

    /// Get the maximum age of the rendered response in seconds
    pub fn max_age(&self) -> Option<u64> {
        self.max_age
    }

    /// Write the freshness to the response headers.
    pub fn write(&self, headers: &mut http::HeaderMap<http::HeaderValue>) {
        let age = self.age();
        headers.insert(http::header::AGE, age.into());
        if let Some(max_age) = self.max_age() {
            headers.insert(
                http::header::CACHE_CONTROL,
                http::HeaderValue::from_str(&format!("max-age={}", max_age)).unwrap(),
            );
        }
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

struct ValidCachedPath {
    full_path: PathBuf,
    timestamp: std::time::SystemTime,
}

impl ValidCachedPath {
    fn try_from_path(value: PathBuf) -> Option<Self> {
        if value.extension() != Some(std::ffi::OsStr::new("html")) {
            return None;
        }
        let timestamp = decode_timestamp(value.file_stem()?.to_str()?)?;
        let full_path = value;
        Some(Self {
            full_path,
            timestamp,
        })
    }

    fn freshness(&self, max_age: Option<std::time::Duration>) -> Option<RenderFreshness> {
        let age = self.timestamp.elapsed().ok()?.as_secs();
        let max_age = max_age.map(|max_age| max_age.as_secs());
        Some(RenderFreshness::new(age, max_age?))
    }
}

fn decode_timestamp(timestamp: &str) -> Option<std::time::SystemTime> {
    let timestamp = u64::from_str_radix(timestamp, 16).ok()?;
    Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp))
}

fn timestamp() -> String {
    let datetime = std::time::SystemTime::now();
    let timestamp = datetime
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{:x}", timestamp)
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
