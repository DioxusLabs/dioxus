//! Incremental file based incremental rendering

#![allow(non_snake_case)]

mod config;
mod freshness;
#[cfg(not(target_arch = "wasm32"))]
mod fs_cache;
mod memory_cache;

use std::time::Duration;

use chrono::Utc;
pub use config::*;
pub use freshness::*;

use self::memory_cache::InMemoryCache;

/// A render that was cached from a previous render.
pub struct CachedRender<'a> {
    /// The route that was rendered
    pub route: String,
    /// The freshness information for the rendered response
    pub freshness: RenderFreshness,
    /// The rendered response
    pub response: &'a [u8],
}

/// An incremental renderer.
pub struct IncrementalRenderer {
    pub(crate) memory_cache: InMemoryCache,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) file_system_cache: fs_cache::FileSystemCache,
    invalidate_after: Option<Duration>,
}

impl IncrementalRenderer {
    /// Create a new incremental renderer builder.
    pub fn builder() -> IncrementalRendererConfig {
        IncrementalRendererConfig::new()
    }

    /// Remove a route from the cache.
    pub fn invalidate(&mut self, route: &str) {
        self.memory_cache.invalidate(route);
        #[cfg(not(target_arch = "wasm32"))]
        self.file_system_cache.invalidate(route);
    }

    /// Remove all routes from the cache.
    pub fn invalidate_all(&mut self) {
        self.memory_cache.clear();
        #[cfg(not(target_arch = "wasm32"))]
        self.file_system_cache.clear();
    }

    /// Cache a rendered response.
    ///
    /// ```rust
    /// # use dioxus_ssr::incremental::IncrementalRenderer;
    /// # let mut renderer = IncrementalRenderer::builder().build();
    /// let route = "/index".to_string();
    /// let response = b"<html><body>Hello world</body></html>";
    /// renderer.cache(route, response).unwrap();
    /// ```
    pub fn cache(
        &mut self,
        route: String,
        html: impl Into<Vec<u8>>,
    ) -> Result<RenderFreshness, IncrementalRendererError> {
        let timestamp = Utc::now();
        let html = html.into();
        #[cfg(not(target_arch = "wasm32"))]
        self.file_system_cache
            .put(route.clone(), timestamp, html.clone())?;
        self.memory_cache.put(route, timestamp, html);
        Ok(RenderFreshness::created_at(
            timestamp,
            self.invalidate_after,
        ))
    }

    /// Try to get a cached response for a route.
    ///
    /// ```rust
    /// # use dioxus_ssr::incremental::IncrementalRenderer;
    /// # let mut renderer = IncrementalRenderer::builder().build();
    /// # let route = "/index".to_string();
    /// # let response = b"<html><body>Hello world</body></html>";
    /// # renderer.cache(route, response).unwrap();
    /// let route = "/index";
    /// let response = renderer.get(route).unwrap();
    /// assert_eq!(response.unwrap().response, b"<html><body>Hello world</body></html>");
    /// ```
    ///
    /// If the route is not cached, `None` is returned.
    ///
    /// ```rust
    /// # use dioxus_ssr::incremental::IncrementalRenderer;
    /// # let mut renderer = IncrementalRenderer::builder().build();
    /// let route = "/index";
    /// let response = renderer.get(route).unwrap();
    /// assert!(response.is_none());
    /// ```
    pub fn get<'a>(
        &'a mut self,
        route: &str,
    ) -> Result<Option<CachedRender<'a>>, IncrementalRendererError> {
        let Self {
            memory_cache,
            file_system_cache,
            ..
        } = self;

        enum FsGetError {
            NotPresent,
            Error(IncrementalRendererError),
        }

        // The borrow checker prevents us from simply using a match/if and returning early. Instead we need to use the more complex closure API
        // non lexical lifetimes will make this possible (it works with polonius)
        let or_insert = || {
            // check the file cache
            #[cfg(not(target_arch = "wasm32"))]
            return match file_system_cache.get(route) {
                Ok(Some((freshness, bytes))) => Ok((freshness.timestamp(), bytes)),
                Ok(None) => Err(FsGetError::NotPresent),
                Err(e) => Err(FsGetError::Error(e)),
            };

            #[allow(unreachable_code)]
            Err(FsGetError::NotPresent)
        };

        match memory_cache.try_get_or_insert(route, or_insert) {
            Ok(Some((freshness, bytes))) => Ok(Some(CachedRender {
                route: route.to_string(),
                freshness,
                response: bytes,
            })),
            Err(FsGetError::NotPresent) | Ok(None) => Ok(None),
            Err(FsGetError::Error(e)) => Err(e),
        }
    }
}

/// An error that can occur while rendering a route or retrieving a cached route.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
