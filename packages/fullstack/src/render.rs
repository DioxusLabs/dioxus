//! A shared pool of renderers for efficient server side rendering.
use crate::render::dioxus_core::NoOpMutations;
use dioxus_ssr::{
    incremental::{CachedRender, RenderFreshness, WrapBody},
    streaming::StreamingRenderer,
    Renderer,
};
use futures_channel::mpsc::Sender;
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::task::block_in_place;
use tokio::task::JoinHandle;

use crate::prelude::*;
use dioxus_lib::prelude::*;

fn spawn_platform<Fut>(f: impl FnOnce() -> Fut + Send + 'static) -> JoinHandle<Fut::Output>
where
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(move || {
            tokio::runtime::Runtime::new()
                .expect("couldn't spawn runtime")
                .block_on(f())
        })
    }
    #[cfg(target_arch = "wasm32")]
    {
        tokio::task::spawn_local(f())
    }
}

struct SsrRendererPool {
    renderers: RwLock<Vec<Renderer>>,
    incremental_cache: Option<RwLock<dioxus_ssr::incremental::IncrementalRenderer>>,
}

impl SsrRendererPool {
    fn new(
        initial_size: usize,
        incremental: Option<dioxus_ssr::incremental::IncrementalRendererConfig>,
    ) -> Self {
        let renderers = RwLock::new((0..initial_size).map(|_| pre_renderer()).collect());
        Self {
            renderers,
            incremental_cache: incremental.map(|cache| RwLock::new(cache.build())),
        }
    }

    fn check_cached_route(
        &self,
        route: &str,
        render_into: &mut Sender<Result<String, dioxus_ssr::incremental::IncrementalRendererError>>,
    ) -> Option<RenderFreshness> {
        if let Some(incremental) = &self.incremental_cache {
            if let Ok(mut incremental) = incremental.write() {
                match incremental.get(route) {
                    Ok(Some(cached_render)) => {
                        let CachedRender {
                            freshness,
                            response,
                            ..
                        } = cached_render;
                        _ = render_into.start_send(String::from_utf8(response.to_vec()).map_err(
                            |err| {
                                dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(
                                    err,
                                ))
                            },
                        ));
                        return Some(freshness);
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to get route \"{route}\" from incremental cache: {e}"
                        );
                    }
                    _ => {}
                }
            }
        }
        None
    }

    async fn render_to(
        self: Arc<Self>,
        cfg: &ServeConfig,
        route: String,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: &DioxusServerContext,
        mut into: Sender<Result<String, dioxus_ssr::incremental::IncrementalRendererError>>,
    ) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
        // before we even spawn anything, we can check synchronously if we have the route cached
        if let Some(freshness) = self.check_cached_route(&route, &mut into) {
            return Ok(freshness);
        }

        let wrapper = FullstackHTMLTemplate {
            cfg: cfg.clone(),
            server_context: server_context.clone(),
        };

        let server_context = server_context.clone();
        let mut renderer = self
            .renderers
            .write()
            .unwrap()
            .pop()
            .unwrap_or_else(pre_renderer);

        let myself = self.clone();

        spawn_platform(move || async move {
            let mut virtual_dom = virtual_dom_factory();

            let mut pre_body = WriteBuffer { buffer: Vec::new() };
            if let Err(err) = wrapper.render_before_body(&mut *pre_body) {
                _ = into.start_send(Err(err));
                return;
            }
            let pre_body = match String::from_utf8(pre_body.buffer) {
                Ok(html) => html,
                Err(err) => {
                    _ = into.start_send(Err(
                        dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(err)),
                    ));
                    return;
                }
            };

            let mut streaming_renderer = StreamingRenderer::new(pre_body, into);

            macro_rules! throw_error {
                ($e:expr) => {
                    streaming_renderer.close_with_error($e);
                    return;
                };
            }

            // poll the future, which may call server_context()
            tracing::info!("Rebuilding vdom");
            with_server_context(server_context.clone(), || {
                block_in_place(|| virtual_dom.rebuild(&mut NoOpMutations));
            });

            // While we are streaming, there is no need to include hydration ids in the SSR render
            renderer.pre_render = false;

            while virtual_dom.suspended_tasks_remaining() {
                ProvideServerContext::new(
                    virtual_dom.wait_for_suspense_work(),
                    server_context.clone(),
                )
                .await;

                with_server_context(server_context.clone(), || {
                    block_in_place(|| virtual_dom.render_suspense_immediate());
                });

                if virtual_dom.suspended_tasks_remaining() {
                    let html = renderer.render(&virtual_dom);
                    streaming_renderer.render(html);
                }
            }
            tracing::info!("Suspense resolved");

            // After suspense is done, we render one last time to get the final html that can be hydrated and then close the body
            let mut post_streaming = WriteBuffer { buffer: Vec::new() };

            // We need to include hydration ids in final the SSR render so that the client can hydrate the correct nodes
            renderer.pre_render = true;
            if let Err(err) = renderer.render_to(&mut post_streaming, &virtual_dom) {
                throw_error!(dioxus_ssr::incremental::IncrementalRendererError::RenderError(err));
            }
            if let Err(err) = wrapper.render_after_body(&mut *post_streaming) {
                throw_error!(err);
            }

            // If incremental rendering is enabled, add the new render to the cache without the streaming bits
            if let Some(incremental) = &self.incremental_cache {
                let mut cached_render = WriteBuffer { buffer: Vec::new() };
                if let Err(err) = wrapper.render_before_body(&mut *cached_render) {
                    throw_error!(err);
                }
                cached_render
                    .buffer
                    .extend_from_slice(post_streaming.buffer.as_slice());

                if let Ok(mut incremental) = incremental.write() {
                    let _ = incremental.cache(route, cached_render.buffer);
                }
            }

            let post_streaming = match String::from_utf8(post_streaming.buffer) {
                Ok(html) => html,
                Err(err) => {
                    throw_error!(dioxus_ssr::incremental::IncrementalRendererError::Other(
                        Box::new(err),
                    ));
                }
            };
            streaming_renderer.finish_streaming(post_streaming);

            myself.renderers.write().unwrap().push(renderer);
        });

        Ok(RenderFreshness::now(None))
    }
}

/// State used in server side rendering. This utilizes a pool of [`dioxus_ssr::Renderer`]s to cache static templates between renders.
#[derive(Clone)]
pub struct SSRState {
    // We keep a pool of renderers to avoid re-creating them on every request. They are boxed to make them very cheap to move
    renderers: Arc<SsrRendererPool>,
}

impl SSRState {
    /// Create a new [`SSRState`].
    pub fn new(cfg: &ServeConfig) -> Self {
        Self {
            renderers: Arc::new(SsrRendererPool::new(4, cfg.incremental.clone())),
        }
    }

    /// Render the application to HTML.
    pub async fn render<'a>(
        &'a self,
        route: String,
        cfg: &'a ServeConfig,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: &'a DioxusServerContext,
        into: Sender<Result<String, dioxus_ssr::incremental::IncrementalRendererError>>,
    ) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
        self.renderers
            .clone()
            .render_to(cfg, route, virtual_dom_factory, server_context, into)
            .await
    }
}

/// The template that wraps the body of the HTML for a fullstack page. This template contains the data needed to hydrate server functions that were run on the server.
#[derive(Default)]
pub struct FullstackHTMLTemplate {
    cfg: ServeConfig,
    server_context: DioxusServerContext,
}

impl FullstackHTMLTemplate {
    /// Create a new [`FullstackHTMLTemplate`].
    pub fn new(cfg: &ServeConfig, server_context: &DioxusServerContext) -> Self {
        Self {
            cfg: cfg.clone(),
            server_context: server_context.clone(),
        }
    }
}

impl dioxus_ssr::incremental::WrapBody for FullstackHTMLTemplate {
    fn render_before_body<R: std::io::Write>(
        &self,
        to: &mut R,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { index, .. } = &self.cfg;

        to.write_all(index.pre_main.as_bytes())?;

        Ok(())
    }

    fn render_after_body<R: std::io::Write>(
        &self,
        to: &mut R,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        // serialize the server state
        crate::html_storage::serialize::encode_in_element(
            &*self.server_context.html_data().map_err(|_| {
                dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new({
                    #[derive(Debug)]
                    struct HTMLDataReadError;

                    impl std::fmt::Display for HTMLDataReadError {
                        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                            f.write_str(
                                "Failed to read the server data to serialize it into the HTML",
                            )
                        }
                    }

                    impl std::error::Error for HTMLDataReadError {}

                    HTMLDataReadError
                }))
            })?,
            to,
        )
        .map_err(|err| dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(err)))?;

        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        {
            // In debug mode, we need to add a script to the page that will reload the page if the websocket disconnects to make full recompile hot reloads work
            let disconnect_js = dioxus_hot_reload::RECONNECT_SCRIPT;

            to.write_all(r#"<script>"#.as_bytes())?;
            to.write_all(disconnect_js.as_bytes())?;
            to.write_all(r#"</script>"#.as_bytes())?;
        }

        let ServeConfig { index, .. } = &self.cfg;

        to.write_all(index.post_main.as_bytes())?;

        Ok(())
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
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

impl std::ops::Deref for WriteBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for WriteBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}
