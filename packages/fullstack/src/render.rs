//! A shared pool of renderers for efficient server side rendering.
use crate::streaming::StreamingRenderer;
use dioxus_interpreter_js::INITIALIZE_STREAMING_JS;
use dioxus_ssr::{
    incremental::{CachedRender, RenderFreshness},
    Renderer,
};
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::sync::Arc;
use std::sync::RwLock;
use std::{collections::HashMap, future::Future};
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
            let document = std::rc::Rc::new(crate::document::server::ServerDocument::default());
            virtual_dom.provide_root_context(document.clone() as std::rc::Rc<dyn Document>);

            // poll the future, which may call server_context()
            tracing::info!("Rebuilding vdom");
            with_server_context(server_context.clone(), || virtual_dom.rebuild_in_place());

            let mut pre_body = String::new();

            if let Err(err) = wrapper.render_head(&mut pre_body, &virtual_dom) {
                _ = into.start_send(Err(err));
                return;
            }

            let stream = Arc::new(StreamingRenderer::new(pre_body, into));
            let scope_to_mount_mapping = Arc::new(RwLock::new(HashMap::new()));

            renderer.pre_render = true;
            {
                let scope_to_mount_mapping = scope_to_mount_mapping.clone();
                let stream = stream.clone();
                renderer.set_render_components(move |renderer, to, vdom, scope| {
                    let is_suspense_boundary =
                        SuspenseContext::downcast_suspense_boundary_from_scope(
                            &vdom.runtime(),
                            scope,
                        )
                        .filter(|s| s.has_suspended_tasks())
                        .is_some();
                    if is_suspense_boundary {
                        let mount = stream.render_placeholder(
                            |to| renderer.render_scope(to, vdom, scope),
                            &mut *to,
                        )?;
                        scope_to_mount_mapping.write().unwrap().insert(scope, mount);
                    } else {
                        renderer.render_scope(to, vdom, scope)?
                    }
                    Ok(())
                });
            }

            macro_rules! throw_error {
                ($e:expr) => {
                    stream.close_with_error($e);
                    return;
                };
            }

            // Render the initial frame with loading placeholders
            let mut initial_frame = renderer.render(&virtual_dom);

            // Along with the initial frame, we render the html after the main element, but before the body tag closes. This should include the script that starts loading the wasm bundle.
            if let Err(err) = wrapper.render_after_main(&mut initial_frame, &virtual_dom) {
                throw_error!(err);
            }
            stream.render(initial_frame);

            // After the initial render, we need to resolve suspense
            while virtual_dom.suspended_tasks_remaining() {
                ProvideServerContext::new(
                    virtual_dom.wait_for_suspense_work(),
                    server_context.clone(),
                )
                .await;
                let resolved_suspense_nodes = ProvideServerContext::new(
                    virtual_dom.render_suspense_immediate(),
                    server_context.clone(),
                )
                .await;

                // Just rerender the resolved nodes
                for scope in resolved_suspense_nodes {
                    let mount = {
                        let mut lock = scope_to_mount_mapping.write().unwrap();
                        lock.remove(&scope)
                    };
                    // If the suspense boundary was immediately removed, it may not have a mount. We can just skip resolving it
                    if let Some(mount) = mount {
                        let mut resolved_chunk = String::new();
                        // After we replace the placeholder in the dom with javascript, we need to send down the resolved data so that the client can hydrate the node
                        let render_suspense = |into: &mut String| {
                            renderer.reset_hydration();
                            renderer.render_scope(into, &virtual_dom, scope)
                        };
                        let resolved_data = serialize_server_data(&virtual_dom, scope);
                        if let Err(err) = stream.replace_placeholder(
                            mount,
                            render_suspense,
                            resolved_data,
                            &mut resolved_chunk,
                        ) {
                            throw_error!(
                                dioxus_ssr::incremental::IncrementalRendererError::RenderError(err)
                            );
                        }

                        stream.render(resolved_chunk);
                    }
                    // Freeze the suspense boundary to prevent future reruns of any child nodes of the suspense boundary
                    if let Some(suspense) = SuspenseContext::downcast_suspense_boundary_from_scope(
                        &virtual_dom.runtime(),
                        scope,
                    ) {
                        suspense.freeze();
                    }
                }
            }

            // After suspense is done, we render the html after the body
            let mut post_streaming = String::new();

            if let Err(err) = wrapper.render_after_body(&mut post_streaming) {
                throw_error!(err);
            }

            // If incremental rendering is enabled, add the new render to the cache without the streaming bits
            if let Some(incremental) = &self.incremental_cache {
                let mut cached_render = String::new();
                if let Err(err) = wrapper.render_head(&mut cached_render, &virtual_dom) {
                    throw_error!(err);
                }
                cached_render.push_str(&post_streaming);

                if let Ok(mut incremental) = incremental.write() {
                    let _ = incremental.cache(route, cached_render);
                }
            }

            stream.render(post_streaming);

            renderer.reset_render_components();
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

fn serialize_server_data(virtual_dom: &VirtualDom, scope: ScopeId) -> String {
    // After we replace the placeholder in the dom with javascript, we need to send down the resolved data so that the client can hydrate the node
    // Extract any data we serialized for hydration (from server futures)
    let html_data =
        crate::html_storage::HTMLData::extract_from_suspense_boundary(virtual_dom, scope);

    // serialize the server state into a base64 string
    html_data.serialized()
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
    /// Render any content before the head of the page.
    pub fn render_head<R: std::fmt::Write>(
        &self,
        to: &mut R,
        virtual_dom: &VirtualDom,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { index, .. } = &self.cfg;

        let title = {
            let document: Option<std::rc::Rc<dyn dioxus_lib::prelude::document::Document>> =
                virtual_dom.in_runtime(|| ScopeId::ROOT.consume_context());
            let document: Option<&crate::document::server::ServerDocument> = document
                .as_ref()
                .and_then(|document| document.as_any().downcast_ref());
            // Collect any head content from the document provider and inject that into the head
            document.and_then(|document| document.title())
        };

        to.write_str(&index.head_before_title)?;
        if let Some(title) = title {
            to.write_str(&title)?;
        } else {
            to.write_str(&index.title)?;
        }
        to.write_str(&index.head_after_title)?;

        let document: Option<std::rc::Rc<dyn dioxus_lib::prelude::document::Document>> =
            virtual_dom.in_runtime(|| ScopeId::ROOT.consume_context());
        let document: Option<&crate::document::server::ServerDocument> = document
            .as_ref()
            .and_then(|document| document.as_any().downcast_ref());
        if let Some(document) = document {
            // Collect any head content from the document provider and inject that into the head
            document.render(to)?;

            // Enable a warning when inserting contents into the head during streaming
            document.start_streaming();
        }

        self.render_before_body(to)?;

        Ok(())
    }

    /// Render any content before the body of the page.
    fn render_before_body<R: std::fmt::Write>(
        &self,
        to: &mut R,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { index, .. } = &self.cfg;

        to.write_str(&index.close_head)?;

        write!(to, "<script>{INITIALIZE_STREAMING_JS}</script>")?;

        Ok(())
    }

    /// Render all content after the main element of the page.
    pub fn render_after_main<R: std::fmt::Write>(
        &self,
        to: &mut R,
        virtual_dom: &VirtualDom,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { index, .. } = &self.cfg;

        // Collect the initial server data from the root node. For most apps, no use_server_futures will be resolved initially, so this will be full on `None`s.
        // Sending down those Nones are still important to tell the client not to run the use_server_futures that are already running on the backend
        let resolved_data = serialize_server_data(virtual_dom, ScopeId::ROOT);
        write!(
            to,
            r#"<script>window.initial_dioxus_hydration_data="{resolved_data}";</script>"#,
        )?;
        to.write_str(&index.post_main)?;

        Ok(())
    }

    /// Render all content after the body of the page.
    pub fn render_after_body<R: std::fmt::Write>(
        &self,
        to: &mut R,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        let ServeConfig { index, .. } = &self.cfg;

        to.write_str(&index.after_closing_body_tag)?;

        Ok(())
    }

    /// Wrap a body in the template
    pub fn wrap_body<R: std::fmt::Write>(
        &self,
        to: &mut R,
        virtual_dom: &VirtualDom,
        body: impl std::fmt::Display,
    ) -> Result<(), dioxus_ssr::incremental::IncrementalRendererError> {
        self.render_head(to, virtual_dom)?;
        write!(to, "{body}")?;
        self.render_after_main(to, virtual_dom)?;
        self.render_after_body(to)?;

        Ok(())
    }
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}
