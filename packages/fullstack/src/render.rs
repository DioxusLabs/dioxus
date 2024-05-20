//! A shared pool of renderers for efficient server side rendering.
use crate::{render::dioxus_core::NoOpMutations, server_context::with_server_context};
use dioxus_ssr::{
    incremental::{RenderFreshness, WrapBody},
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

enum SsrRendererPool {
    Renderer(RwLock<Vec<Renderer>>),
    Incremental(RwLock<Vec<dioxus_ssr::incremental::IncrementalRenderer>>),
}

impl SsrRendererPool {
    async fn render_to(
        self: Arc<Self>,
        cfg: &ServeConfig,
        route: String,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: &DioxusServerContext,
        into: Sender<String>,
    ) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
        let wrapper = FullstackRenderer {
            cfg: cfg.clone(),
            server_context: server_context.clone(),
        };
        match &*self {
            Self::Renderer(pool) => {
                let server_context = server_context.clone();
                let mut renderer = pool.write().unwrap().pop().unwrap_or_else(pre_renderer);

                let (tx, rx) = tokio::sync::oneshot::channel();

                let myself = self.clone();
                spawn_platform(move || async move {
                    let mut virtual_dom = virtual_dom_factory();

                    let mut pre_body = WriteBuffer { buffer: Vec::new() };
                    if let Err(err) = wrapper.render_before_body(&mut *pre_body) {
                        _ = tx.send(err);
                        return;
                    }
                    let pre_body = match String::from_utf8(pre_body.buffer) {
                        Ok(html) => html,
                        Err(err) => {
                            _ = tx.send(dioxus_ssr::incremental::IncrementalRendererError::Other(
                                Box::new(err),
                            ));
                            return;
                        }
                    };

                    let mut streaming_renderer = StreamingRenderer::new(pre_body, into);

                    // poll the future, which may call server_context()
                    tracing::info!("Rebuilding vdom");
                    with_server_context(server_context.clone(), || {
                        block_in_place(|| virtual_dom.rebuild(&mut NoOpMutations));
                    });
                    virtual_dom.rebuild(&mut NoOpMutations);

                    // While we are streaming, there is no need to include hydration ids in the SSR render
                    renderer.pre_render = false;

                    loop {
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
                        } else {
                            break;
                        }
                    }
                    tracing::info!("Suspense resolved");

                    // After suspense is done, we render one last time to get the final html that can be hydrated and then close the body
                    let mut post_streaming = WriteBuffer { buffer: Vec::new() };

                    // We need to include hydration ids in final the SSR render so that the client can hydrate the correct nodes
                    renderer.pre_render = true;
                    if let Err(err) = renderer.render_to(&mut post_streaming, &virtual_dom) {
                        _ = tx.send(
                            dioxus_ssr::incremental::IncrementalRendererError::RenderError(err),
                        );
                        return;
                    }
                    if let Err(err) = wrapper.render_after_body(&mut *post_streaming) {
                        _ = tx.send(err);
                        return;
                    }
                    let post_streaming = match String::from_utf8(post_streaming.buffer) {
                        Ok(html) => html,
                        Err(err) => {
                            _ = tx.send(dioxus_ssr::incremental::IncrementalRendererError::Other(
                                Box::new(err),
                            ));
                            return;
                        }
                    };
                    streaming_renderer.finish_streaming(post_streaming);

                    if let Self::Renderer(pool) = &*myself {
                        pool.write().unwrap().push(renderer);
                    }
                });

                Ok(RenderFreshness::now(None))
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
                                    // poll the future, which may call server_context()
                                    tracing::info!("Rebuilding vdom");
                                    with_server_context(server_context.clone(), || {
                                        block_in_place(|| vdom.rebuild(&mut NoOpMutations));
                                    });
                                    ProvideServerContext::new(
                                        vdom.wait_for_suspense(),
                                        server_context,
                                    )
                                    .await;
                                    tracing::info!("Suspense resolved");
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
                let (freshness, _) = rx.await.unwrap()?;

                Ok(freshness)
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
        into: Sender<String>,
    ) -> Result<RenderFreshness, dioxus_ssr::incremental::IncrementalRendererError> {
        self.renderers
            .clone()
            .render_to(cfg, route, virtual_dom_factory, server_context, into)
            .await
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
            // This is copied from the Dioxus-CLI package at ../packages/cli
            let disconnect_js = r#"(function () {
              var protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
              var url = protocol + "//" + window.location.host + "/_dioxus/ws";
              var poll_interval = 8080;
            
              var reload_upon_connect = (event) => {
                // Firefox will send a 1001 code when the connection is closed because the page is reloaded
                // Only firefox will trigger the onclose event when the page is reloaded manually: https://stackoverflow.com/questions/10965720/should-websocket-onclose-be-triggered-by-user-navigation-or-refresh
                // We should not reload the page in this case
                if (event.code === 1001) {
                  return;
                }
                window.setTimeout(() => {
                  var ws = new WebSocket(url);
                  ws.onopen = () => window.location.reload();
                  ws.onclose = reload_upon_connect;
                }, poll_interval);
              };
            
              var ws = new WebSocket(url);
            
              ws.onmessage = (ev) => {
                console.log("Received message: ", ev, ev.data);
            
                if (ev.data == "reload") {
                  window.location.reload();
                }
              };
            
              ws.onclose = reload_upon_connect;
            })();"#;

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
