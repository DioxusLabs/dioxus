//! A shared pool of renderers for efficient server side rendering.
use crate::render::dioxus_core::NoOpMutations;
use crate::server_context::SERVER_CONTEXT;
use dioxus_lib::prelude::VirtualDom;
use dioxus_ssr::{
    incremental::{IncrementalRendererConfig, RenderFreshness, WrapBody},
    Renderer,
};
use std::future::Future;
use std::sync::Arc;
use std::sync::RwLock;
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

enum SsrRendererPool {
    Renderer(RwLock<Vec<Renderer>>),
    Incremental(RwLock<Vec<dioxus_ssr::incremental::IncrementalRenderer>>),
}

impl SsrRendererPool {
    async fn render_to(
        &self,
        cfg: &ServeConfig,
        route: String,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: &DioxusServerContext,
    ) -> Result<(RenderFreshness, String), dioxus_ssr::incremental::IncrementalRendererError> {
        let wrapper = FullstackRenderer {
            cfg: cfg.clone(),
            server_context: server_context.clone(),
        };
        match self {
            Self::Renderer(pool) => {
                let server_context = Box::new(server_context.clone());
                let mut renderer = pool.write().unwrap().pop().unwrap_or_else(pre_renderer);

                let (tx, rx) = tokio::sync::oneshot::channel();

                spawn_platform(move || async move {
                    let mut vdom = virtual_dom_factory();
                    let mut to = WriteBuffer { buffer: Vec::new() };
                    // before polling the future, we need to set the context
                    let prev_context = SERVER_CONTEXT.with(|ctx| ctx.replace(server_context));
                    // poll the future, which may call server_context()
                    tracing::info!("Rebuilding vdom");
                    vdom.rebuild(&mut NoOpMutations);
                    vdom.wait_for_suspense().await;
                    tracing::info!("Suspense resolved");
                    // after polling the future, we need to restore the context
                    SERVER_CONTEXT.with(|ctx| ctx.replace(prev_context));

                    if let Err(err) = wrapper.render_before_body(&mut *to) {
                        let _ = tx.send(Err(err));
                        return;
                    }
                    if let Err(err) = renderer.render_to(&mut to, &vdom) {
                        let _ = tx.send(Err(
                            dioxus_ssr::incremental::IncrementalRendererError::RenderError(err),
                        ));
                        return;
                    }
                    if let Err(err) = wrapper.render_after_body(&mut *to) {
                        let _ = tx.send(Err(err));
                        return;
                    }
                    match String::from_utf8(to.buffer) {
                        Ok(html) => {
                            let _ = tx.send(Ok((renderer, RenderFreshness::now(None), html)));
                        }
                        Err(err) => {
                            _ = tx.send(Err(
                                dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(
                                    err,
                                )),
                            ));
                        }
                    }
                });
                let (renderer, freshness, html) = rx.await.unwrap()?;
                pool.write().unwrap().push(renderer);
                Ok((freshness, html))
            }
            Self::Incremental(pool) => {
                let mut renderer =
                    pool.write().unwrap().pop().unwrap_or_else(|| {
                        incremental_pre_renderer(cfg.incremental.as_ref().unwrap())
                    });

                let (tx, rx) = tokio::sync::oneshot::channel();

                let server_context = server_context.clone();
                spawn_platform(move || async move {
                    let mut to = WriteBuffer { buffer: Vec::new() };
                    match renderer
                        .render(
                            route,
                            virtual_dom_factory,
                            &mut *to,
                            |vdom| {
                                Box::pin(async move {
                                    // before polling the future, we need to set the context
                                    let prev_context = SERVER_CONTEXT
                                        .with(|ctx| ctx.replace(Box::new(server_context)));
                                    // poll the future, which may call server_context()
                                    tracing::info!("Rebuilding vdom");
                                    vdom.rebuild(&mut NoOpMutations);
                                    vdom.wait_for_suspense().await;
                                    tracing::info!("Suspense resolved");
                                    // after polling the future, we need to restore the context
                                    SERVER_CONTEXT.with(|ctx| ctx.replace(prev_context));
                                })
                            },
                            &wrapper,
                        )
                        .await
                    {
                        Ok(freshness) => {
                            match String::from_utf8(to.buffer).map_err(|err| {
                                dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(
                                    err,
                                ))
                            }) {
                                Ok(html) => {
                                    let _ = tx.send(Ok((freshness, html)));
                                }
                                Err(err) => {
                                    let _ = tx.send(Err(err));
                                }
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(Err(err));
                        }
                    }
                });
                let (freshness, html) = rx.await.unwrap()?;

                Ok((freshness, html))
            }
        }
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
        if cfg.incremental.is_some() {
            return Self {
                renderers: Arc::new(SsrRendererPool::Incremental(RwLock::new(vec![
                    incremental_pre_renderer(cfg.incremental.as_ref().unwrap()),
                    incremental_pre_renderer(cfg.incremental.as_ref().unwrap()),
                    incremental_pre_renderer(cfg.incremental.as_ref().unwrap()),
                    incremental_pre_renderer(cfg.incremental.as_ref().unwrap()),
                ]))),
            };
        }

        Self {
            renderers: Arc::new(SsrRendererPool::Renderer(RwLock::new(vec![
                pre_renderer(),
                pre_renderer(),
                pre_renderer(),
                pre_renderer(),
            ]))),
        }
    }

    /// Render the application to HTML.
    pub async fn render<'a>(
        &'a self,
        route: String,
        cfg: &'a ServeConfig,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: &'a DioxusServerContext,
    ) -> Result<RenderResponse, dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { .. } = cfg;

        let (freshness, html) = self
            .renderers
            .render_to(cfg, route, virtual_dom_factory, server_context)
            .await?;

        Ok(RenderResponse { html, freshness })
    }
}

struct FullstackRenderer {
    cfg: ServeConfig,
    server_context: DioxusServerContext,
}

impl dioxus_ssr::incremental::WrapBody for FullstackRenderer {
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
        // serialize the props
        // TODO: restore props serialization
        // crate::html_storage::serialize::encode_props_in_element(&self.cfg.props, to).map_err(
        //     |err| dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(err)),
        // )?;
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
            let disconnect_js = r#"(function () {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = protocol + '//' + window.location.host + '/_dioxus/disconnect';
    const poll_interval = 1000;
    const reload_upon_connect = () => {
        console.log('Disconnected from server. Attempting to reconnect...');
        window.setTimeout(
            () => {
                // Try to reconnect to the websocket
                const ws = new WebSocket(url);
                ws.onopen = () => {
                    // If we reconnect, reload the page
                    window.location.reload();
                }
                // Otherwise, try again in a second
                reload_upon_connect();
            },
            poll_interval);
    };

    // on initial page load connect to the disconnect ws
    const ws = new WebSocket(url);
    // if we disconnect, start polling
    ws.onclose = reload_upon_connect;
})()"#;

            to.write_all(r#"<script>"#.as_bytes())?;
            to.write_all(disconnect_js.as_bytes())?;
            to.write_all(r#"</script>"#.as_bytes())?;
        }

        let ServeConfig { index, .. } = &self.cfg;

        to.write_all(index.post_main.as_bytes())?;

        Ok(())
    }
}

/// A rendered response from the server.
#[derive(Debug)]
pub struct RenderResponse {
    pub(crate) html: String,
    pub(crate) freshness: RenderFreshness,
}

impl RenderResponse {
    /// Get the rendered HTML.
    pub fn html(&self) -> &str {
        &self.html
    }

    /// Get the freshness of the rendered HTML.
    pub fn freshness(&self) -> RenderFreshness {
        self.freshness
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}

fn incremental_pre_renderer(
    cfg: &IncrementalRendererConfig,
) -> dioxus_ssr::incremental::IncrementalRenderer {
    let mut renderer = cfg.clone().build();
    renderer.renderer_mut().pre_render = true;
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
