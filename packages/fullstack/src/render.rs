//! A shared pool of renderers for efficient server side rendering.
use crate::{
    document::ServerDocument, stream::StreamingResponse, template::serialize_server_data,
    ServeConfig,
};
use crate::{
    streaming::{Mount, StreamingRenderer},
    template::FullstackHTMLTemplate,
};
use dioxus_interpreter_js::INITIALIZE_STREAMING_JS;
use dioxus_isrg::{
    CachedRender, IncrementalRenderer, IncrementalRendererError, IsrConfig as IsrgConfig,
    RenderFreshness,
};
use dioxus_lib::document::Document;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use std::sync::Arc;
use std::sync::RwLock;
use std::{collections::HashMap, future::Future};
use tokio::task::JoinHandle;

use crate::prelude::*;
use dioxus_lib::prelude::*;

/// State used in server side rendering. This utilizes a pool of [`dioxus_ssr::Renderer`]s to cache static templates between renders.
#[derive(Clone)]
pub struct SSRState {
    // We keep a pool of renderers to avoid re-creating them on every request. They are boxed to make them very cheap to move
    renderers: Arc<SsrRenderer>,
}

impl SSRState {
    /// Create a new [`SSRState`].
    pub fn new(cfg: &ServeConfig) -> Self {
        Self {
            renderers: Arc::new(SsrRenderer::new(4, cfg.incremental.clone())),
        }
    }

    /// Render the application to HTML.
    pub async fn render<'a>(
        &'a self,
        route: String,
        cfg: ServeConfig,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: DioxusServerContext,
    ) -> Result<StreamingResponse, IncrementalRendererError> {
        self.renderers
            .clone()
            .render_to(cfg, route, virtual_dom_factory, server_context)
            .await
    }
}

struct SsrRenderer {
    renderers: RwLock<Vec<Renderer>>,
    incremental_cache: Option<RwLock<IncrementalRenderer>>,
}

impl SsrRenderer {
    fn new(initial_size: usize, incremental: Option<IsrgConfig>) -> Self {
        let renderers = RwLock::new((0..initial_size).map(|_| pre_renderer()).collect());
        let incremental_cache = incremental.map(|cache| RwLock::new(cache.build()));

        Self {
            renderers,
            incremental_cache,
        }
    }

    /// Look for a cached route in the incremental cache and send it into the render channel if it exists
    fn check_cached_route(
        &self,
        route: &str,
        render_into: &mut Sender<Result<String, IncrementalRendererError>>,
    ) -> Option<RenderFreshness> {
        let incremental = self.incremental_cache.as_ref()?;
        let mut incremental = incremental.write().ok()?;

        let cached = incremental.get(route).ok().flatten()?;

        _ = render_into.start_send(
            String::from_utf8(cached.response.to_vec())
                .map_err(|err| IncrementalRendererError::Other(Box::new(err))),
        );

        Some(cached.freshness)
    }

    /// Render a virtual dom into a stream. This method will return immediately and continue streaming the result in the background
    /// The streaming is canceled when the stream the function returns is dropped
    async fn render_to(
        self: Arc<Self>,
        cfg: ServeConfig,
        route: String,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
        server_context: DioxusServerContext,
    ) -> Result<StreamingResponse, IncrementalRendererError> {
        let (mut into, rx) =
            futures_channel::mpsc::channel::<Result<String, IncrementalRendererError>>(1000);

        // before we even spawn anything, we can check synchronously if we have the route cached
        if let Some(freshness) = self.check_cached_route(&route, &mut into) {
            return Ok(StreamingResponse::new(rx, freshness, None));
        }

        let wrapper = FullstackHTMLTemplate { cfg };

        let join_handle = spawn_platform(move || {
            self.respond(virtual_dom_factory, server_context, wrapper, into, route)
        });

        Ok(StreamingResponse::new(
            rx,
            RenderFreshness::now(None),
            Some(join_handle),
        ))
    }

    async fn respond(
        self: Arc<Self>,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + '_,
        server_context: DioxusServerContext,
        wrapper: FullstackHTMLTemplate,
        mut sender: Sender<Result<String, IncrementalRendererError>>,
        route: String,
    ) {
        let mut renderer = self
            .renderers
            .write()
            .unwrap()
            .pop()
            .unwrap_or_else(pre_renderer);

        let mut virtual_dom = virtual_dom_factory();

        let document = std::rc::Rc::new(ServerDocument::default());
        virtual_dom.provide_root_context(document.clone());
        virtual_dom.provide_root_context(document.clone() as std::rc::Rc<dyn Document>);
        server_context.run_with(|| virtual_dom.rebuild_in_place());

        let mut pre_body = String::new();

        if let Err(err) = wrapper.render_head(&mut pre_body, &virtual_dom) {
            _ = sender.start_send(Err(err));
            return;
        }

        let stream = Arc::new(StreamingRenderer::start(pre_body, sender));
        let scope_to_mount_mapping = Arc::new(RwLock::new(HashMap::new()));
        renderer.pre_render = true;

        // We use a stack to keep track of what suspense boundaries we are nested in to add children to the correct boundary
        // The stack starts with the root scope because the root is a suspense boundary
        renderer.set_render_components({
            let scope_to_mount_mapping = scope_to_mount_mapping.clone();
            let stream = stream.clone();
            let pending_suspense_boundaries_stack = RwLock::new(vec![]);

            move |renderer, to, vdom, scope| {
                let is_suspense_boundary =
                    SuspenseContext::downcast_suspense_boundary_from_scope(&vdom.runtime(), scope)
                        .filter(|s| s.has_suspended_tasks())
                        .is_some();

                if !is_suspense_boundary {
                    renderer.render_scope(to, vdom, scope)?;
                    return Ok(());
                }

                let mount = stream.render_placeholder(
                    |to| {
                        {
                            pending_suspense_boundaries_stack
                                .write()
                                .unwrap()
                                .push(scope);
                        }
                        let out = renderer.render_scope(to, vdom, scope);
                        {
                            pending_suspense_boundaries_stack.write().unwrap().pop();
                        }
                        out
                    },
                    &mut *to,
                )?;

                // Add the suspense boundary to the list of pending suspense boundaries
                // We will replace the mount with the resolved contents later once the suspense boundary is resolved
                let mut scope_to_mount_mapping_write = scope_to_mount_mapping.write().unwrap();
                scope_to_mount_mapping_write.insert(
                    scope,
                    PendingSuspenseBoundary {
                        mount,
                        children: vec![],
                    },
                );

                // Add the scope to the list of children of the parent suspense boundary
                let pending_suspense_boundaries_stack =
                    pending_suspense_boundaries_stack.read().unwrap();

                // If there is a parent suspense boundary, add the scope to the list of children
                // This suspense boundary will start capturing errors when the parent is resolved
                if let Some(parent) = pending_suspense_boundaries_stack.last() {
                    let parent = scope_to_mount_mapping_write.get_mut(parent).unwrap();
                    parent.children.push(scope);
                } else {
                    // Otherwise this is a root suspense boundary, so we need to start capturing errors immediately
                    vdom.in_runtime(|| {
                        start_capturing_errors(scope);
                    });
                }

                Ok(())
            }
        });

        let post_streaming = self
            .clone()
            .unqueue_suspense(
                &mut renderer,
                virtual_dom,
                wrapper,
                &stream,
                server_context,
                scope_to_mount_mapping,
                route,
            )
            .await;

        let post_streaming = match post_streaming {
            Ok(post_streaming) => post_streaming,
            Err(err) => {
                stream.close_with_error(err);
                return;
            }
        };

        stream.render(post_streaming);
        renderer.reset_render_components();

        self.renderers.write().unwrap().push(renderer);
    }

    async fn unqueue_suspense(
        self: Arc<Self>,
        renderer: &mut Renderer,
        mut virtual_dom: VirtualDom,
        wrapper: FullstackHTMLTemplate,
        stream: &Arc<StreamingRenderer<IncrementalRendererError>>,
        server_context: DioxusServerContext,
        scope_to_mount_mapping: Arc<RwLock<HashMap<ScopeId, PendingSuspenseBoundary>>>,
        route: String,
    ) -> Result<String, IncrementalRendererError> {
        // Render the initial frame with loading placeholders
        let mut initial_frame = renderer.render(&virtual_dom);

        // Along with the initial frame, we render the html after the main element, but before the body tag closes. This should include the script that starts loading the wasm bundle.
        wrapper.render_after_main(&mut initial_frame, &virtual_dom)?;
        stream.render(initial_frame);

        // After the initial render, we need to resolve suspense
        while virtual_dom.suspended_tasks_remaining() {
            ProvideServerContext::new(virtual_dom.wait_for_suspense_work(), server_context.clone())
                .await;
            let resolved_suspense_nodes = ProvideServerContext::new(
                virtual_dom.render_suspense_immediate(),
                server_context.clone(),
            )
            .await;

            // Just rerender the resolved nodes
            for scope in resolved_suspense_nodes {
                let pending_suspense_boundary =
                    scope_to_mount_mapping.write().unwrap().remove(&scope);

                // If the suspense boundary was immediately removed, it may not have a mount. We can just skip resolving it
                let Some(pending_suspense_boundary) = pending_suspense_boundary else {
                    continue;
                };

                let mut resolved_chunk = String::new();

                // After we replace the placeholder in the dom with javascript, we need to send down the resolved data so that the client can hydrate the node
                let render_suspense = |into: &mut String| {
                    renderer.reset_hydration();
                    renderer.render_scope(into, &virtual_dom, scope)
                };
                let resolved_data = serialize_server_data(&virtual_dom, scope);

                stream
                    .replace_placeholder(
                        pending_suspense_boundary.mount,
                        render_suspense,
                        resolved_data,
                        &mut resolved_chunk,
                    )
                    .map_err(|err| dioxus_isrg::IncrementalRendererError::RenderError(err))?;

                stream.render(resolved_chunk);

                // Freeze the suspense boundary to prevent future reruns of any child nodes of the suspense boundary
                if let Some(suspense) = SuspenseContext::downcast_suspense_boundary_from_scope(
                    &virtual_dom.runtime(),
                    scope,
                ) {
                    suspense.freeze();

                    // Go to every child suspense boundary and add an error boundary. Since we cannot rerun any nodes above the child suspense boundary,
                    // we need to capture the errors and send them to the client as it resolves
                    virtual_dom.in_runtime(|| {
                        for &suspense_scope in pending_suspense_boundary.children.iter() {
                            start_capturing_errors(suspense_scope);
                        }
                    });
                }
            }
        }

        // After suspense is done, we render the html after the body
        let mut post_streaming = String::new();
        wrapper.render_after_body(&mut post_streaming)?;

        // If incremental rendering is enabled, add the new render to the cache without the streaming bits
        if let Some(incremental) = &self.incremental_cache {
            let mut cached_render = String::new();
            wrapper.render_head(&mut cached_render, &virtual_dom)?;
            cached_render.push_str(&post_streaming);

            if let Ok(mut incremental) = incremental.write() {
                let _ = incremental.cache(route, cached_render);
            }
        }

        Ok(post_streaming)
    }
}

/// Start capturing errors at a suspense boundary. If the parent suspense boundary is frozen, we need to capture the errors in the suspense boundary
/// and send them to the client to continue bubbling up
fn start_capturing_errors(suspense_scope: ScopeId) {
    // Add an error boundary to the scope
    suspense_scope.in_runtime(provide_error_boundary);
}

fn pre_renderer() -> Renderer {
    let mut renderer = Renderer::default();
    renderer.pre_render = true;
    renderer
}

/// Spawn a task in the background. If wasm is enabled, this will use the single threaded tokio runtime
fn spawn_platform<Fut>(f: impl FnOnce() -> Fut + Send + 'static) -> JoinHandle<Fut::Output>
where
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    #[cfg(target_arch = "wasm32")]
    {
        tokio::task::spawn_local(f())
    }

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
}

/// A suspense boundary that is pending with a placeholder in the client
struct PendingSuspenseBoundary {
    mount: Mount,
    children: Vec<ScopeId>,
}
