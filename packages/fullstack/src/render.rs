//! A shared pool of renderers for efficient server side rendering.
use crate::render::dioxus_core::NoOpMutations;
use dioxus_ssr::{
    incremental::{CachedRender, RenderFreshness},
    streaming::StreamingRenderer,
    Renderer,
};
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::sync::Arc;
use std::sync::RwLock;
use std::{future::Future, time::Duration};
use tokio::{task::JoinHandle, time::Instant};

use crate::prelude::*;
use dioxus_lib::prelude::*;

fn spawn_platform<Fut>(f: impl FnOnce() -> Fut + Send + 'static) -> JoinHandle<Fut::Output>
where
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        use tokio_util::task::LocalPoolHandle;
        static TASK_POOL: std::sync::OnceLock<LocalPoolHandle> = std::sync::OnceLock::new();

        let pool = TASK_POOL.get_or_init(|| {
            let threads = std::thread::available_parallelism()
                .unwrap_or(std::num::NonZeroUsize::new(1).unwrap());
            LocalPoolHandle::new(threads.into())
        });

        pool.spawn_pinned(f)
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
    ) -> Result<
        (
            RenderFreshness,
            impl Stream<Item = Result<String, dioxus_ssr::incremental::IncrementalRendererError>>,
        ),
        dioxus_ssr::incremental::IncrementalRendererError,
    > {
        struct ReceiverWithDrop {
            receiver: futures_channel::mpsc::Receiver<
                Result<String, dioxus_ssr::incremental::IncrementalRendererError>,
            >,
            cancel_task: Option<tokio::task::JoinHandle<()>>,
        }

        impl Stream for ReceiverWithDrop {
            type Item = Result<String, dioxus_ssr::incremental::IncrementalRendererError>;

            fn poll_next(
                mut self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Self::Item>> {
                self.receiver.poll_next_unpin(cx)
            }
        }

        // When we drop the stream, we need to cancel the task that is feeding values to the stream
        impl Drop for ReceiverWithDrop {
            fn drop(&mut self) {
                if let Some(cancel_task) = self.cancel_task.take() {
                    cancel_task.abort();
                }
            }
        }

        let (mut into, rx) = futures_channel::mpsc::channel::<
            Result<String, dioxus_ssr::incremental::IncrementalRendererError>,
        >(1000);

        // before we even spawn anything, we can check synchronously if we have the route cached
        if let Some(freshness) = self.check_cached_route(&route, &mut into) {
            return Ok((
                freshness,
                ReceiverWithDrop {
                    receiver: rx,
                    cancel_task: None,
                },
            ));
        }

        let wrapper = FullstackHTMLTemplate { cfg: cfg.clone() };

        let stream_page = cfg.stream_page;
        let server_context = server_context.clone();
        let mut renderer = self
            .renderers
            .write()
            .unwrap()
            .pop()
            .unwrap_or_else(pre_renderer);

        let myself = self.clone();

        let join_handle = spawn_platform(move || async move {
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
                virtual_dom.rebuild(&mut NoOpMutations);
            });

            // While we are streaming, there is no need to include hydration ids in the SSR render
            renderer.pre_render = false;

            // We only render with a maximum frequency of 200ms to avoid forcing the client to render too much data
            const RENDER_DEDUPLICATE_TIMEOUT: Duration = Duration::from_millis(200);

            let mut last_render: Option<Instant> = None;

            while virtual_dom.suspended_tasks_remaining() {
                let deadline = last_render
                    .map(|last_render| {
                        RENDER_DEDUPLICATE_TIMEOUT
                            .checked_sub(last_render.elapsed())
                            .unwrap_or(Duration::ZERO)
                    })
                    .unwrap_or(tokio::time::Duration::MAX);

                let run_virtual_dom = async {
                    ProvideServerContext::new(
                        virtual_dom.wait_for_suspense_work(),
                        server_context.clone(),
                    )
                    .await;

                    with_server_context(server_context.clone(), || {
                        virtual_dom.render_suspense_immediate();
                    });
                };

                let mut rerender = |last_render: &mut Option<Instant>, virtual_dom: &VirtualDom| {
                    if virtual_dom.suspended_tasks_remaining() {
                        let html = renderer.render(virtual_dom);
                        if stream_page {
                            streaming_renderer.render(html);
                        }
                        *last_render = Some(Instant::now());
                    }
                };

                tokio::select! {
                    // If it has been 100ms since running a scope, we should stream the edits to the client
                    _ = tokio::time::sleep(deadline) => {
                        rerender(&mut last_render, &virtual_dom);
                    }
                    // Otherwise, just keep running the virtual dom or quit if suspense is done
                    _ = run_virtual_dom => {
                        if last_render.is_none() {
                            rerender(&mut last_render, &virtual_dom);
                        }
                    }
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

            // Extract any data we serialized for hydration (from server futures)
            let html_data = crate::html_storage::HTMLData::extract_from_virtual_dom(&virtual_dom);

            if let Err(err) = wrapper.render_after_body(&mut *post_streaming, &html_data) {
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

        Ok((
            RenderFreshness::now(None),
            ReceiverWithDrop {
                receiver: rx,
                cancel_task: Some(join_handle),
            },
        ))
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
    ) -> Result<
        (
            RenderFreshness,
            impl Stream<Item = Result<String, dioxus_ssr::incremental::IncrementalRendererError>>,
        ),
        dioxus_ssr::incremental::IncrementalRendererError,
    > {
        self.renderers
            .clone()
            .render_to(cfg, route, virtual_dom_factory, server_context)
            .await
    }
}

/// The template that wraps the body of the HTML for a fullstack page. This template contains the data needed to hydrate server functions that were run on the server.
#[derive(Default)]
pub struct FullstackHTMLTemplate {
    cfg: ServeConfig,
}

impl FullstackHTMLTemplate {
    /// Create a new [`FullstackHTMLTemplate`].
    pub fn new(cfg: &ServeConfig) -> Self {
        Self { cfg: cfg.clone() }
    }
}

impl FullstackHTMLTemplate {
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
        html_data: &crate::html_storage::HTMLData,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        // serialize the server state
        crate::html_storage::serialize::encode_in_element(html_data, to).map_err(|err| {
            dioxus_ssr::incremental::IncrementalRendererError::Other(Box::new(err))
        })?;

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
