//! A shared pool of renderers for efficient server side rendering.

use crate::isrg::{
    CachedRender, IncrementalRenderer, IncrementalRendererConfig, IncrementalRendererError,
    RenderFreshness,
};
use crate::streaming::{Mount, StreamingRenderer};
use crate::{document::ServerDocument, ServeConfig};
use dioxus_cli_config::base_path;
use dioxus_core::{
    consume_context, has_context, try_consume_context, DynamicNode, ErrorContext, Runtime, ScopeId,
    SuspenseContext, TemplateNode, VNode, VirtualDom,
};
use dioxus_fullstack_core::{history::provide_fullstack_history_context, HttpError, ServerFnError};
use dioxus_fullstack_core::{FullstackContext, StreamingStatus};
use dioxus_fullstack_core::{HydrationContext, SerializedHydrationData};
use dioxus_router::ParseRouteError;
use dioxus_ssr::Renderer;
use futures_channel::mpsc::Sender;
use futures_util::{Stream, StreamExt};
use http::{request::Parts, HeaderMap, StatusCode};
use std::{
    collections::HashMap,
    fmt::Write,
    iter::Peekable,
    rc::Rc,
    sync::{Arc, RwLock},
};
use tokio_util::task::LocalPoolHandle;

use crate::StreamingMode;

/// Errors that can occur during server side rendering before the initial chunk is sent down
pub enum SSRError {
    /// An error from the incremental renderer. This should result in a 500 code
    Incremental(IncrementalRendererError),

    HttpError {
        status: StatusCode,
        message: Option<String>,
    },
}

/// A suspense boundary that is pending with a placeholder in the client
struct PendingSuspenseBoundary {
    mount: Mount,
    children: Vec<ScopeId>,
}

pub(crate) struct SsrRendererPool {
    renderers: RwLock<Vec<Renderer>>,
    incremental_cache: Option<RwLock<IncrementalRenderer>>,
}

impl SsrRendererPool {
    pub(crate) fn new(initial_size: usize, incremental: Option<IncrementalRendererConfig>) -> Self {
        let renderers = RwLock::new((0..initial_size).map(|_| Self::pre_renderer()).collect());
        Self {
            renderers,
            incremental_cache: incremental.map(|cache| RwLock::new(cache.build())),
        }
    }

    /// Look for a cached route in the incremental cache and send it into the render channel if it exists
    fn check_cached_route(
        &self,
        route: &str,
        render_into: &mut Sender<Result<String, IncrementalRendererError>>,
    ) -> Option<RenderFreshness> {
        let incremental = self.incremental_cache.as_ref()?;

        if let Ok(mut incremental) = incremental.write() {
            match incremental.get(route) {
                Ok(Some(cached_render)) => {
                    let CachedRender {
                        freshness,
                        response,
                        ..
                    } = cached_render;
                    _ = render_into.start_send(
                        String::from_utf8(response.to_vec())
                            .map_err(|err| IncrementalRendererError::Other(err.into())),
                    );
                    return Some(freshness);
                }
                Err(e) => {
                    tracing::error!("Failed to get route \"{route}\" from incremental cache: {e}");
                }
                _ => {}
            }
        }

        None
    }

    /// Render a virtual dom into a stream. This method will return immediately and continue streaming the result in the background
    /// The streaming is canceled when the stream the function returns is dropped
    pub(crate) async fn render_to(
        self: Arc<Self>,
        parts: Parts,
        cfg: &ServeConfig,
        rt: &LocalPoolHandle,
        virtual_dom_factory: impl FnOnce() -> VirtualDom + Send + Sync + 'static,
    ) -> Result<
        (
            HttpError,
            HeaderMap,
            RenderFreshness,
            impl Stream<Item = Result<String, IncrementalRendererError>>,
        ),
        SSRError,
    > {
        struct ReceiverWithDrop {
            receiver: futures_channel::mpsc::Receiver<Result<String, IncrementalRendererError>>,
            cancel_task: Option<tokio::task::JoinHandle<()>>,
        }

        impl Stream for ReceiverWithDrop {
            type Item = Result<String, IncrementalRendererError>;

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

        let route = parts
            .uri
            .path_and_query()
            .ok_or_else(|| SSRError::HttpError {
                status: StatusCode::BAD_REQUEST,
                message: None,
            })?
            .to_string();

        let (mut into, rx) =
            futures_channel::mpsc::channel::<Result<String, IncrementalRendererError>>(1000);

        let (initial_result_tx, initial_result_rx) = futures_channel::oneshot::channel();

        // before we even spawn anything, we can check synchronously if we have the route cached
        if let Some(freshness) = self.check_cached_route(&route, &mut into) {
            return Ok((
                HttpError {
                    status: StatusCode::OK,
                    message: None,
                },
                HeaderMap::new(),
                freshness,
                ReceiverWithDrop {
                    receiver: rx,
                    cancel_task: None,
                },
            ));
        }

        let mut renderer = self
            .renderers
            .write()
            .unwrap()
            .pop()
            .unwrap_or_else(Self::pre_renderer);

        let myself = self.clone();
        let streaming_mode = cfg.streaming_mode;

        let cfg = cfg.clone();
        let create_render_future = move || async move {
            let mut virtual_dom = virtual_dom_factory();
            let document = Rc::new(ServerDocument::default());
            virtual_dom.provide_root_context(document.clone());

            // If there is a base path, trim the base path from the route and add the base path formatting to the
            // history provider
            let history = if let Some(base_path) = base_path() {
                let base_path = base_path.trim_matches('/');
                let base_path = format!("/{base_path}");
                let route = route.strip_prefix(&base_path).unwrap_or(&route);
                dioxus_history::MemoryHistory::with_initial_path(route).with_prefix(base_path)
            } else {
                dioxus_history::MemoryHistory::with_initial_path(&route)
            };

            // Provide the document and streaming context to the root of the app
            let streaming_context =
                virtual_dom.in_scope(ScopeId::ROOT, || FullstackContext::new(parts));
            virtual_dom.provide_root_context(document.clone() as Rc<dyn dioxus_document::Document>);
            virtual_dom.provide_root_context(streaming_context.clone());

            virtual_dom.in_scope(ScopeId::ROOT, || {
                // Wrap the memory history in a fullstack history provider to provide the initial route for hydration
                provide_fullstack_history_context(history);

                // Provide a hydration compatible error boundary that serializes errors for the client
                dioxus_core::provide_create_error_boundary(
                    dioxus_fullstack_core::init_error_boundary,
                );
            });

            // rebuild the virtual dom
            virtual_dom.rebuild_in_place();

            // If streaming is disabled, wait for the virtual dom to finish all suspense work
            // before rendering anything
            if streaming_mode == StreamingMode::Disabled {
                virtual_dom.wait_for_suspense().await;
            } else {
                // Otherwise, just wait for the streaming context to signal the initial chunk is ready
                loop {
                    // Check if the router has finished and set the streaming context to finished
                    let streaming_context_finished = virtual_dom
                        .in_scope(ScopeId::ROOT, || streaming_context.streaming_state())
                        == StreamingStatus::InitialChunkCommitted;

                    // Or if this app isn't using the router and has finished suspense
                    let suspense_finished = !virtual_dom.suspended_tasks_remaining();
                    if streaming_context_finished || suspense_finished {
                        break;
                    }

                    // Wait for new async work that runs during suspense (mainly use_server_futures)
                    virtual_dom.wait_for_suspense_work().await;

                    // Do that async work
                    virtual_dom.render_suspense_immediate().await;
                }
            }

            // check if there are any errors from the root error boundary
            let error = virtual_dom.in_scope(ScopeId::ROOT_ERROR_BOUNDARY, || {
                consume_context::<ErrorContext>().error()
            });

            if let Some(error) = error {
                let mut status_code = None;
                let mut out_message = None;

                // If the errors include an `HttpError` or `StatusCode` or `ServerFnError`, we need
                // to try and return the appropriate status code
                if let Some(error) = error.downcast_ref::<HttpError>() {
                    status_code = Some(error.status);
                    out_message = error.message.clone();
                }

                if let Some(error) = error.downcast_ref::<StatusCode>() {
                    status_code = Some(*error);
                }

                // todo - the user is allowed to return anything that impls `From<ServerFnError>`
                // we need to eventually be able to downcast that and get the status code from it
                if let Some(ServerFnError::ServerError { message, code, .. }) = error.downcast_ref()
                {
                    status_code = Some(
                        (*code)
                            .try_into()
                            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                    );

                    out_message = Some(message.clone());
                }

                // If there was an error while routing, return the error with a 404 status
                // Return a routing error if any of the errors were a routing error
                if let Some(routing_error) = error.downcast_ref::<ParseRouteError>().cloned() {
                    status_code = Some(StatusCode::NOT_FOUND);
                    out_message = Some(routing_error.to_string());
                }

                // If we captured anything that produces a status code, we should return that status code.
                if let Some(status_code) = status_code {
                    _ = initial_result_tx.send(Err(SSRError::HttpError {
                        status: status_code,
                        message: out_message,
                    }));
                    return;
                }

                _ = initial_result_tx.send(Err(SSRError::Incremental(
                    IncrementalRendererError::Other(error),
                )));

                return;
            }

            // Check the FullstackContext in case the user set the statuscode manually or via a layout.
            let http_status = streaming_context.current_http_status();
            let headers = streaming_context
                .take_response_headers()
                .unwrap_or_default();

            // Now that we handled any errors from rendering, we can send the initial ok result
            _ = initial_result_tx.send(Ok((http_status, headers)));

            // Wait long enough to assemble the `<head>` of the document before starting to stream
            let mut pre_body = String::new();
            if let Err(err) = Self::render_head(&cfg, &mut pre_body, &virtual_dom) {
                _ = into.start_send(Err(err));
                return;
            }

            let stream = Arc::new(StreamingRenderer::new(pre_body, into));
            let scope_to_mount_mapping = Arc::new(RwLock::new(HashMap::new()));

            renderer.pre_render = true;
            {
                let scope_to_mount_mapping = scope_to_mount_mapping.clone();
                let stream = stream.clone();
                renderer.set_render_components(Self::streaming_render_component_callback(
                    stream,
                    scope_to_mount_mapping,
                ));
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
            if let Err(err) = Self::render_after_main(&cfg, &mut initial_frame, &virtual_dom) {
                throw_error!(err);
            }
            stream.render(initial_frame);

            // After the initial render, we need to resolve suspense
            while virtual_dom.suspended_tasks_remaining() {
                virtual_dom.wait_for_suspense_work().await;
                let resolved_suspense_nodes = virtual_dom.render_suspense_immediate().await;

                // Just rerender the resolved nodes
                for scope in resolved_suspense_nodes {
                    let pending_suspense_boundary = {
                        let mut lock = scope_to_mount_mapping.write().unwrap();
                        lock.remove(&scope)
                    };
                    // If the suspense boundary was immediately removed, it may not have a mount. We can just skip resolving it
                    if let Some(pending_suspense_boundary) = pending_suspense_boundary {
                        let mut resolved_chunk = String::new();
                        // After we replace the placeholder in the dom with javascript, we need to send down the resolved data so that the client can hydrate the node
                        let render_suspense = |into: &mut String| {
                            renderer.reset_hydration();
                            renderer.render_scope(into, &virtual_dom, scope)
                        };
                        let resolved_data = Self::serialize_server_data(&virtual_dom, scope);
                        if let Err(err) = stream.replace_placeholder(
                            pending_suspense_boundary.mount,
                            render_suspense,
                            resolved_data,
                            &mut resolved_chunk,
                        ) {
                            throw_error!(IncrementalRendererError::RenderError(err));
                        }

                        stream.render(resolved_chunk);
                        // Freeze the suspense boundary to prevent future reruns of any child nodes of the suspense boundary
                        if let Some(suspense) =
                            SuspenseContext::downcast_suspense_boundary_from_scope(
                                &virtual_dom.runtime(),
                                scope,
                            )
                        {
                            suspense.freeze();
                            // Go to every child suspense boundary and add an error boundary. Since we cannot rerun any nodes above the child suspense boundary,
                            // we need to capture the errors and send them to the client as it resolves
                            virtual_dom.in_runtime(|| {
                                for &suspense_scope in pending_suspense_boundary.children.iter() {
                                    Self::start_capturing_errors(suspense_scope);
                                }
                            });
                        }
                    }
                }
            }

            // After suspense is done, we render the html after the body
            let mut post_streaming = String::new();

            if let Err(err) = Self::render_after_body(&cfg, &mut post_streaming) {
                throw_error!(err);
            }

            // If incremental rendering is enabled, add the new render to the cache without the streaming bits
            if let Some(incremental) = &self.incremental_cache {
                let mut cached_render = String::new();
                if let Err(err) = Self::render_head(&cfg, &mut cached_render, &virtual_dom) {
                    throw_error!(err);
                }
                renderer.reset_hydration();
                if let Err(err) = renderer.render_to(&mut cached_render, &virtual_dom) {
                    throw_error!(IncrementalRendererError::RenderError(err));
                }
                if let Err(err) = Self::render_after_main(&cfg, &mut cached_render, &virtual_dom) {
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
        };

        // Spawn the render future onto the local pool
        let join_handle = rt.spawn_pinned(create_render_future);

        // Wait for the initial result which determines the status code
        let (status, headers) = initial_result_rx
            .await
            .map_err(|err| SSRError::Incremental(IncrementalRendererError::Other(err.into())))??;

        Ok((
            status,
            headers,
            RenderFreshness::now(None),
            ReceiverWithDrop {
                receiver: rx,
                cancel_task: Some(join_handle),
            },
        ))
    }

    fn pre_renderer() -> Renderer {
        let mut renderer = Renderer::default();
        renderer.pre_render = true;
        renderer
    }

    /// Create the streaming render component callback. It will keep track of what scopes are mounted to what pending
    /// suspense boundaries in the DOM.
    ///
    /// This mapping is used to replace the DOM mount with the resolved contents once the suspense boundary is finished.
    fn streaming_render_component_callback(
        stream: Arc<StreamingRenderer<IncrementalRendererError>>,
        scope_to_mount_mapping: Arc<RwLock<HashMap<ScopeId, PendingSuspenseBoundary>>>,
    ) -> impl Fn(&mut Renderer, &mut dyn Write, &VirtualDom, ScopeId) -> std::fmt::Result
           + Send
           + Sync
           + 'static {
        // We use a stack to keep track of what suspense boundaries we are nested in to add children to the correct boundary
        // The stack starts with the root scope because the root is a suspense boundary
        let pending_suspense_boundaries_stack = RwLock::new(vec![]);
        move |renderer, to, vdom, scope| {
            let is_suspense_boundary =
                SuspenseContext::downcast_suspense_boundary_from_scope(&vdom.runtime(), scope)
                    .filter(|s| s.has_suspended_tasks())
                    .is_some();
            if is_suspense_boundary {
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
                }
                // Otherwise this is a root suspense boundary, so we need to start capturing errors immediately
                else {
                    vdom.in_runtime(|| {
                        Self::start_capturing_errors(scope);
                    });
                }
            } else {
                renderer.render_scope(to, vdom, scope)?
            }
            Ok(())
        }
    }

    /// Start capturing errors at a suspense boundary. If the parent suspense boundary is frozen, we need to capture the errors in the suspense boundary
    /// and send them to the client to continue bubbling up
    fn start_capturing_errors(suspense_scope: ScopeId) {
        // Add an error boundary to the scope. We serialize the suspense error boundary separately so we can use
        // the normal in memory ErrorContext here
        Runtime::current().in_scope(suspense_scope, || {
            dioxus_core::provide_context(ErrorContext::new(None))
        });
    }

    fn serialize_server_data(virtual_dom: &VirtualDom, scope: ScopeId) -> SerializedHydrationData {
        // After we replace the placeholder in the dom with javascript, we need to send down the resolved data so that the client can hydrate the node
        // Extract any data we serialized for hydration (from server futures)
        let html_data = Self::extract_from_suspense_boundary(virtual_dom, scope);

        // serialize the server state into a base64 string
        html_data.serialized()
    }

    /// Walks through the suspense boundary in a depth first order and extracts the data from the context API.
    /// We use depth first order instead of relying on the order the hooks are called in because during suspense on the server, the order that futures are run in may be non deterministic.
    pub(crate) fn extract_from_suspense_boundary(
        vdom: &VirtualDom,
        scope: ScopeId,
    ) -> HydrationContext {
        let data = HydrationContext::default();
        Self::serialize_errors(&data, vdom, scope);
        Self::take_from_scope(&data, vdom, scope);
        data
    }

    /// Get the errors from the suspense boundary
    fn serialize_errors(context: &HydrationContext, vdom: &VirtualDom, scope: ScopeId) {
        // If there is an error boundary on the suspense boundary, grab the error from the context API
        // and throw it on the client so that it bubbles up to the nearest error boundary
        let error = vdom.in_scope(scope, || {
            try_consume_context::<ErrorContext>().and_then(|error_context| error_context.error())
        });
        context
            .error_entry()
            .insert(&error, std::panic::Location::caller());
    }

    fn take_from_scope(context: &HydrationContext, vdom: &VirtualDom, scope: ScopeId) {
        vdom.in_scope(scope, || {
            // Grab any serializable server context from this scope
            if let Some(other) = has_context::<HydrationContext>() {
                context.extend(&other);
            }
        });

        // then continue to any children
        if let Some(scope) = vdom.get_scope(scope) {
            // If this is a suspense boundary, move into the children first (even if they are suspended) because that will be run first on the client
            if let Some(suspense_boundary) =
                SuspenseContext::downcast_suspense_boundary_from_scope(&vdom.runtime(), scope.id())
            {
                if let Some(node) = suspense_boundary.suspended_nodes() {
                    Self::take_from_vnode(context, vdom, &node);
                }
            }
            if let Some(node) = scope.try_root_node() {
                Self::take_from_vnode(context, vdom, node);
            }
        }
    }

    fn take_from_vnode(context: &HydrationContext, vdom: &VirtualDom, vnode: &VNode) {
        let template = &vnode.template;
        let mut dynamic_nodes_iter = template.node_paths.iter().copied().enumerate().peekable();
        for (root_idx, node) in template.roots.iter().enumerate() {
            match node {
                TemplateNode::Element { .. } => {
                    // dioxus core runs nodes in an odd order to not mess up template order. We need to match
                    // that order here
                    let (start, end) =
                        match Self::collect_dyn_node_range(&mut dynamic_nodes_iter, root_idx as u8)
                        {
                            Some((a, b)) => (a, b),
                            None => continue,
                        };

                    let reversed_iter = (start..=end).rev();

                    for dynamic_node_id in reversed_iter {
                        let dynamic_node = &vnode.dynamic_nodes[dynamic_node_id];
                        Self::take_from_dynamic_node(
                            context,
                            vdom,
                            vnode,
                            dynamic_node,
                            dynamic_node_id,
                        );
                    }
                }
                TemplateNode::Dynamic { id } => {
                    // Take a dynamic node off the depth first iterator
                    _ = dynamic_nodes_iter.next().unwrap();
                    let dynamic_node = &vnode.dynamic_nodes[*id];
                    Self::take_from_dynamic_node(context, vdom, vnode, dynamic_node, *id);
                }
                _ => {}
            }
        }
    }

    fn take_from_dynamic_node(
        context: &HydrationContext,
        vdom: &VirtualDom,
        vnode: &VNode,
        dyn_node: &DynamicNode,
        dynamic_node_index: usize,
    ) {
        match dyn_node {
            DynamicNode::Component(comp) => {
                if let Some(scope) = comp.mounted_scope(dynamic_node_index, vnode, vdom) {
                    Self::take_from_scope(context, vdom, scope.id());
                }
            }
            DynamicNode::Fragment(nodes) => {
                for node in nodes {
                    Self::take_from_vnode(context, vdom, node);
                }
            }
            _ => {}
        }
    }

    // This should have the same behavior as the collect_dyn_node_range method in core
    // Find the index of the first and last dynamic node under a root index
    fn collect_dyn_node_range(
        dynamic_nodes: &mut Peekable<impl Iterator<Item = (usize, &'static [u8])>>,
        root_idx: u8,
    ) -> Option<(usize, usize)> {
        let start = match dynamic_nodes.peek() {
            Some((idx, [first, ..])) if *first == root_idx => *idx,
            _ => return None,
        };

        let mut end = start;

        while let Some((idx, p)) =
            dynamic_nodes.next_if(|(_, p)| matches!(p, [idx, ..] if *idx == root_idx))
        {
            if p.len() == 1 {
                continue;
            }

            end = idx;
        }

        Some((start, end))
    }

    /// Render any content before the head of the page.
    pub fn render_head<R: std::fmt::Write>(
        cfg: &ServeConfig,
        to: &mut R,
        virtual_dom: &VirtualDom,
    ) -> Result<(), IncrementalRendererError> {
        let title = {
            let document: Option<Rc<ServerDocument>> =
                virtual_dom.in_scope(ScopeId::ROOT, dioxus_core::try_consume_context);
            // Collect any head content from the document provider and inject that into the head
            document.and_then(|document| document.title())
        };

        to.write_str(&cfg.index.head_before_title)?;
        if let Some(title) = title {
            to.write_str(&title)?;
        } else {
            to.write_str(&cfg.index.title)?;
        }
        to.write_str(&cfg.index.head_after_title)?;

        let document =
            virtual_dom.in_scope(ScopeId::ROOT, try_consume_context::<Rc<ServerDocument>>);
        if let Some(document) = document {
            // Collect any head content from the document provider and inject that into the head
            document.render(to)?;

            // Enable a warning when inserting contents into the head during streaming
            document.start_streaming();
        }

        Self::render_before_body(cfg, to)?;

        Ok(())
    }

    /// Render any content before the body of the page.
    fn render_before_body<R: std::fmt::Write>(
        cfg: &ServeConfig,
        to: &mut R,
    ) -> Result<(), IncrementalRendererError> {
        to.write_str(&cfg.index.close_head)?;

        // // #[cfg(feature = "document")]
        // {
        use dioxus_interpreter_js::INITIALIZE_STREAMING_JS;
        write!(to, "<script>{INITIALIZE_STREAMING_JS}</script>")?;
        // }

        Ok(())
    }

    /// Render all content after the main element of the page.
    pub fn render_after_main<R: std::fmt::Write>(
        cfg: &ServeConfig,
        to: &mut R,
        virtual_dom: &VirtualDom,
    ) -> Result<(), IncrementalRendererError> {
        // Collect the initial server data from the root node. For most apps, no use_server_futures will be resolved initially, so this will be full on `None`s.
        // Sending down those Nones are still important to tell the client not to run the use_server_futures that are already running on the backend
        let resolved_data = SsrRendererPool::serialize_server_data(virtual_dom, ScopeId::ROOT);
        // We always send down the data required to hydrate components on the client
        let raw_data = resolved_data.data;
        write!(
            to,
            r#"<script>window.initial_dioxus_hydration_data="{raw_data}";"#,
        )?;
        #[cfg(debug_assertions)]
        {
            // In debug mode, we also send down the type names and locations of the serialized data
            let debug_types = &resolved_data.debug_types;
            let debug_locations = &resolved_data.debug_locations;
            write!(
                to,
                r#"window.initial_dioxus_hydration_debug_types={debug_types};"#,
            )?;
            write!(
                to,
                r#"window.initial_dioxus_hydration_debug_locations={debug_locations};"#,
            )?;
        }
        write!(to, r#"</script>"#,)?;
        to.write_str(&cfg.index.post_main)?;

        Ok(())
    }

    /// Render all content after the body of the page.
    pub fn render_after_body<R: std::fmt::Write>(
        cfg: &ServeConfig,
        to: &mut R,
    ) -> Result<(), IncrementalRendererError> {
        to.write_str(&cfg.index.after_closing_body_tag)?;

        Ok(())
    }
}
