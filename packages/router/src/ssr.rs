//! Incremental file based incremental rendering

#![allow(non_snake_case)]

use crate::prelude::*;
use dioxus::prelude::*;
use std::{
    io::{Read, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Something that can render a HTML page from a body.
pub trait RenderHTML {
    /// Render a HTML page from a body.
    fn render_html(&self, body: &str) -> String;
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
    fn render_html(&self, body: &str) -> String {
        format!("{}{}{}", self.before_body, body, self.after_body)
    }
}

/// A configuration for the incremental renderer.
pub struct IncrementalRendererConfig<R: RenderHTML> {
    static_dir: PathBuf,
    memory_cache_limit: usize,
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
            memory_cache_limit: 100,
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

    /// Build the incremental renderer.
    pub fn build(self) -> IncrementalRenderer<R> {
        IncrementalRenderer {
            static_dir: self.static_dir,
            memory_cache: NonZeroUsize::new(self.memory_cache_limit)
                .map(|limit| lru::LruCache::new(limit)),
            render: self.render,
        }
    }
}

/// An incremental renderer.
pub struct IncrementalRenderer<R: RenderHTML> {
    static_dir: PathBuf,
    memory_cache: Option<lru::LruCache<String, String>>,
    render: R,
}

impl<R: RenderHTML> IncrementalRenderer<R> {
    /// Create a new incremental renderer builder.
    pub fn builder(renderer: R) -> IncrementalRendererConfig<R> {
        IncrementalRendererConfig::new(renderer)
    }

    fn render_uncached<Rt>(&self, route: Rt) -> String
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
    {
        let mut vdom = VirtualDom::new_with_props(RenderPath, RenderPathProps { path: route });
        let _ = vdom.rebuild();

        let body = dioxus_ssr::render(&vdom);

        self.render.render_html(&body)
    }

    fn add_to_cache(&mut self, route: String, html: String) {
        let file_path = self.route_as_path(&route);
        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).unwrap();
            }
        }
        let file = std::fs::File::create(dbg!(file_path)).unwrap();
        let mut file = std::io::BufWriter::new(file);
        file.write_all(html.as_bytes()).unwrap();
        self.add_to_memory_cache(route, html);
    }

    fn add_to_memory_cache<K: AsRef<str> + ToString, V: ToString>(&mut self, route: K, html: V) {
        if let Some(cache) = self.memory_cache.as_mut() {
            if cache.contains(route.as_ref()) {
                cache.promote(route.as_ref())
            } else {
                cache.put(route.to_string(), html.to_string());
            }
        }
    }

    fn search_cache(&mut self, route: String) -> Option<String> {
        if let Some(cache_hit) = self
            .memory_cache
            .as_mut()
            .and_then(|cache| cache.get(&route).cloned())
        {
            Some(cache_hit)
        } else {
            let file_path = self.route_as_path(&route);
            if let Ok(file) = std::fs::File::open(file_path) {
                let mut file = std::io::BufReader::new(file);
                let mut html = String::new();
                file.read_to_string(&mut html).ok()?;
                self.add_to_memory_cache(route, html.clone());
                Some(html)
            } else {
                None
            }
        }
    }

    /// Render a route or get it from cache.
    pub fn render<Rt>(&mut self, route: Rt) -> String
    where
        Rt: Routable,
        <Rt as FromStr>::Err: std::fmt::Display,
    {
        // check if this route is cached
        if let Some(html) = self.search_cache(route.to_string()) {
            return html;
        }

        // if not, create it
        println!("cache miss");
        let html = self.render_uncached(route.clone());
        self.add_to_cache(route.to_string(), html.clone());

        html
    }

    fn route_as_path(&self, route: &str) -> PathBuf {
        let mut file_path = self.static_dir.clone();
        for segment in route.split('/') {
            file_path.push(segment);
        }
        file_path.push("index");
        file_path.set_extension("html");
        file_path
    }

    /// Pre-cache all static routes.
    pub fn pre_cache_static<Rt>(&mut self)
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
                        let _ = self.render(route);
                    }
                    Err(e) => {
                        log::error!("Error pre-caching static route: {}", e);
                    }
                }
            }
        }
    }
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
