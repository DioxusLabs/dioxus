//! Incremental file based incremental rendering

#![allow(non_snake_case)]

use crate::prelude::*;
use dioxus::prelude::*;
use rustc_hash::FxHasher;
use std::{
    hash::BuildHasherDefault,
    io::Write,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, SystemTime},
};

/// Something that can render a HTML page from a body.
pub trait RenderHTML {
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

impl RenderHTML for DefaultRenderer {
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
pub struct IncrementalRendererConfig<R: RenderHTML> {
    static_dir: PathBuf,
    memory_cache_limit: usize,
    invalidate_after: Option<Duration>,
    render: R,
}

impl Default for IncrementalRendererConfig<DefaultRenderer> {
    fn default() -> Self {
        Self::new(DefaultRenderer::default())
    }
}

impl<R: RenderHTML> IncrementalRendererConfig<R> {
    /// Create a new incremental renderer configuration.
    pub fn new(render: R) -> Self {
        Self {
            static_dir: PathBuf::from("./static"),
            memory_cache_limit: 10000,
            invalidate_after: None,
            render,
        }
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
    pub fn build(self) -> IncrementalRenderer<R> {
        IncrementalRenderer {
            static_dir: self.static_dir,
            memory_cache: NonZeroUsize::new(self.memory_cache_limit)
                .map(|limit| lru::LruCache::with_hasher(limit, Default::default())),
            invalidate_after: self.invalidate_after,
            render: self.render,
            ssr_renderer: dioxus_ssr::Renderer::new(),
        }
    }
}

/// An incremental renderer.
pub struct IncrementalRenderer<R: RenderHTML> {
    static_dir: PathBuf,
    memory_cache:
        Option<lru::LruCache<String, (SystemTime, Vec<u8>), BuildHasherDefault<FxHasher>>>,
    invalidate_after: Option<Duration>,
    ssr_renderer: dioxus_ssr::Renderer,
    render: R,
}

impl<R: RenderHTML> IncrementalRenderer<R> {
    /// Create a new incremental renderer builder.
    pub fn builder(renderer: R) -> IncrementalRendererConfig<R> {
        IncrementalRendererConfig::new(renderer)
    }

    fn track_timestamps(&self) -> bool {
        self.invalidate_after.is_some()
    }

    fn render_and_cache<Rt>(
        &mut self,
        route: Rt,
        output: &mut impl Write,
    ) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
    {
        let route_str = route.to_string();
        let mut vdom = VirtualDom::new_with_props(RenderPath, RenderPathProps { path: route });
        let _ = vdom.rebuild();

        let mut html_buffer = WriteBuffer { buffer: Vec::new() };
        self.render.render_before_body(&mut html_buffer)?;
        self.ssr_renderer.render_to(&mut html_buffer, &mut vdom)?;
        self.render.render_after_body(&mut html_buffer)?;
        let html_buffer = html_buffer.buffer;

        output.write_all(&html_buffer)?;

        self.add_to_cache(route_str, html_buffer)
    }

    fn add_to_cache(
        &mut self,
        route: String,
        html: Vec<u8>,
    ) -> Result<(), IncrementalRendererError> {
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
        Ok(())
    }

    fn add_to_memory_cache(&mut self, route: String, html: Vec<u8>) {
        if let Some(cache) = self.memory_cache.as_mut() {
            cache.put(route.to_string(), (SystemTime::now(), html));
        }
    }

    fn promote_memory_cache<K: AsRef<str>>(&mut self, route: K) {
        if let Some(cache) = self.memory_cache.as_mut() {
            cache.promote(route.as_ref())
        }
    }

    fn search_cache(
        &mut self,
        route: String,
        output: &mut impl Write,
    ) -> Result<bool, IncrementalRendererError> {
        if let Some((timestamp, cache_hit)) = self
            .memory_cache
            .as_mut()
            .and_then(|cache| cache.get(&route))
        {
            if let (Ok(elapsed), Some(invalidate_after)) =
                (timestamp.elapsed(), self.invalidate_after)
            {
                if elapsed < invalidate_after {
                    log::trace!("memory cache hit {:?}", route);
                    output.write_all(cache_hit)?;
                    return Ok(true);
                }
            } else {
                log::trace!("memory cache hit {:?}", route);
                output.write_all(cache_hit)?;
                return Ok(true);
            }
        }
        if let Some(file_path) = self.find_file(&route) {
            if let Ok(file) = std::fs::File::open(file_path.full_path) {
                let mut file = std::io::BufReader::new(file);
                std::io::copy(&mut file, output)?;
                log::trace!("file cache hit {:?}", route);
                self.promote_memory_cache(&route);
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Render a route or get it from cache.
    pub fn render<Rt>(
        &mut self,
        route: Rt,
        output: &mut impl Write,
    ) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
    {
        // check if this route is cached
        if !self.search_cache(route.to_string(), output)? {
            // if not, create it
            self.render_and_cache(route, output)?;
            log::trace!("cache miss");
        }

        Ok(())
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
        let mut file_path = self.static_dir.clone();
        for segment in route.split('/') {
            file_path.push(segment);
        }
        if self.track_timestamps() {
            file_path.push("index");
            file_path.push(timestamp());
        } else {
            file_path.push("index");
        }
        file_path.set_extension("html");
        file_path
    }

    /// Pre-cache all static routes.
    pub fn pre_cache_static_routes<Rt>(&mut self) -> Result<(), IncrementalRendererError>
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
    {
        for route in Rt::SITE_MAP
            .iter()
            .flat_map(|seg| seg.flatten().into_iter())
        {
            // check if this is a static segment
            let mut is_static = true;
            let mut full_path = String::new();
            for segment in &route {
                match segment {
                    SegmentType::Static(s) => {
                        full_path += "/";
                        full_path += s;
                    }
                    _ => {
                        // skip routes with any dynamic segments
                        is_static = false;
                        break;
                    }
                }
            }

            if is_static {
                match Rt::from_str(&full_path) {
                    Ok(route) => {
                        let _ = self.render(route, &mut std::io::sink())?;
                    }
                    Err(e) => {
                        log::error!("Error pre-caching static route: {}", e);
                    }
                }
            }
        }

        Ok(())
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

impl Write for WriteBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.flush()
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
        let timestamp = decode_timestamp(&value.file_stem()?.to_str()?)?;
        let full_path = value;
        Some(Self {
            full_path,
            timestamp,
        })
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

#[inline_props]
fn RenderPath<R>(cx: Scope, path: R) -> Element
where
    R: Routable,
    <R as FromStr>::Err: std::fmt::Display,
{
    let path = path.clone();
    render! {
        GenericRouter::<R> {
            config: || RouterConfig::default().history(MemoryHistory::with_initial_path(path))
        }
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
}
