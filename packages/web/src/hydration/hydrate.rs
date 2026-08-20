//! When hydrating streaming components:
//! 1. Hydrate the already-rendered DOM on the outside
//! 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
//! 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

use crate::dom::WebsysDom;
use dioxus_core::{ScopeState, SuspenseBoundaryProps, VirtualDom};
use dioxus_fullstack_core::HydrationContext;
use futures_channel::mpsc::UnboundedReceiver;
use wasm_bindgen::JsCast;

use super::cursor::HydrationCursor;

use super::SuspenseMessage;
use super::suspense::path_to_resolved_suspense_id;

fn children_array(parent: &web_sys::Node) -> js_sys::Array {
    js_sys::Array::from(parent.child_nodes().unchecked_ref())
}

fn node_array(node: &web_sys::Node) -> js_sys::Array {
    let array = js_sys::Array::new();
    array.push(node.unchecked_ref());
    array
}

#[derive(Debug)]
pub(crate) enum RehydrationError {
    /// The client tried to rehydrate a vnode before the dom was built
    VNodeNotInitialized,
    /// The client tried to rehydrate a suspense boundary that was not mounted on the server
    SuspenseHydrationIdNotFound,
    /// The client tried to rehydrate a dom id that was not found on the server
    ElementNotFound,
    /// The server-rendered DOM did not match the shape the client expected
    /// while walking the VDOM (e.g. a tag, text node, or split offset mismatch).
    HydrationMismatch,
}

impl WebsysDom {
    pub fn rehydrate_streaming(&mut self, message: SuspenseMessage, dom: &mut VirtualDom) {
        if let Err(err) = self.rehydrate_streaming_inner(message, dom) {
            tracing::error!("Rehydration failed. {:?}", err);
        }
    }

    fn rehydrate_streaming_inner(
        &mut self,
        message: SuspenseMessage,
        dom: &mut VirtualDom,
    ) -> Result<(), RehydrationError> {
        let SuspenseMessage {
            suspense_path,
            data,
            #[cfg(debug_assertions)]
            debug_types,
            #[cfg(debug_assertions)]
            debug_locations,
        } = message;
        let document = web_sys::window().unwrap().document().unwrap();
        // Before we start rehydrating the suspense boundary we need to check that the suspense boundary exists. It may have been removed on the client.
        let resolved_suspense_id = path_to_resolved_suspense_id(&suspense_path);
        let resolved_suspense_element = document
            .get_element_by_id(&resolved_suspense_id)
            .ok_or(RehydrationError::ElementNotFound)?;

        // First convert the dom id into a scope id based on the discovery order of the suspense boundaries.
        // This may fail if the id is not parsable, or if the suspense boundary was removed after partial hydration on the client.
        let id = self
            .suspense_hydration_ids
            .get_suspense_boundary(&suspense_path)
            .ok_or(RehydrationError::SuspenseHydrationIdNotFound)?;
        // Snapshot the new nodes. `resolve_suspense` pushes them after it
        // pushes the placeholder target so replacement stays stack-only.
        let children = children_array(resolved_suspense_element.unchecked_ref());
        let children_len = children.length();

        // A zero-DOM stream still needs core's suspense placement. Push one
        // empty text node through that placement, then let the hydration cursor
        // claim it if the mounted scope has an addressable empty slot.
        let mut empty_hydration_root = None;

        #[cfg(not(debug_assertions))]
        let debug_types = None;
        #[cfg(not(debug_assertions))]
        let debug_locations = None;

        let server_data = HydrationContext::from_serialized(&data, debug_types, debug_locations);
        // If the server streamed an error into this boundary, there is no
        // resolved subtree to hydrate: the error bubbles to the nearest
        // ErrorBoundary, whose output replaces the boundary entirely, so the
        // markerless walk would mismatch the resolved suspense scope against
        // that error DOM. Throw the error (which marks the ErrorBoundary dirty),
        // drop the streamed nodes, and let that boundary render its message on
        // the next render instead of hydrating here.
        if let Some(error) = server_data.error_entry().get().ok().flatten() {
            resolved_suspense_element.remove();
            dom.in_runtime(|| dom.runtime().throw_error(id, error));
            return Ok(());
        }

        server_data.in_context(|| {
            // rerun the scope with the new data
            SuspenseBoundaryProps::resolve_suspense(id, dom, self, |to| {
                // `resolve_suspense` queues a `push_id` for the node being
                // replaced. The streamed nodes below are pushed directly onto
                // the JS interpreter stack, so flush first to keep the target
                // below the replacement nodes for `replace_with`.
                to.flush_edits();
                if children_len == 0 {
                    let node: web_sys::Node = document.create_text_node("").unchecked_into();
                    to.interpreter.base().push_root(node.clone());
                    empty_hydration_root = Some(node);
                    1
                } else {
                    for index in 0..children_len {
                        let node: web_sys::Node = children.get(index).unchecked_into();
                        to.interpreter.base().push_root(node);
                    }
                    children_len as usize
                }
            });
        });

        // Flush the mutations that will swap the placeholder nodes with the resolved nodes
        self.flush_edits();

        // Remove the streaming div
        resolved_suspense_element.remove();

        let Some(root_scope) = dom.get_scope(id) else {
            // If the scope was removed on the client, we may not be able to rehydrate it, but this shouldn't cause an error
            return Ok(());
        };

        // As we hydrate the suspense boundary, set the current path to the path of the suspense boundary
        self.suspense_hydration_ids
            .current_path
            .clone_from(&suspense_path);

        let roots = empty_hydration_root
            .as_ref()
            .map(node_array)
            .unwrap_or(children);
        self.start_hydration_at_scope(root_scope, dom, roots, false, true)?;

        Ok(())
    }

    fn start_hydration_at_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        under: js_sys::Array,
        filter_scripts: bool,
        collect_suspense: bool,
    ) -> Result<(), RehydrationError> {
        // Park on the first server-rendered root; subsequent roots are stepped
        // via `advance(1)` as the VDOM walker descends siblings. When SSR emitted
        // no roots, hydrate directly inside the mount element so zero-DOM root ids
        // still get anchors with a concrete parent.
        let mut cursor =
            match HydrationCursor::over_roots(self.interpreter.base(), under, filter_scripts)? {
                Some(cursor) => cursor,
                None => HydrationCursor::in_parent(self.interpreter.base(), self.root.clone()),
            };
        self.emit_scope(scope, dom, &mut cursor, collect_suspense)
    }

    pub fn rehydrate(
        &mut self,
        vdom: &mut VirtualDom,
    ) -> Result<UnboundedReceiver<SuspenseMessage>, RehydrationError> {
        let (mut tx, rx) = futures_channel::mpsc::unbounded();
        // A single registration path for both build profiles. The JS side always
        // invokes the callback with four arguments; in release the trailing
        // type/location metadata arrives as `null` (the server omits it) and is
        // dropped before the message is sent.
        let closure = move |path: Vec<u32>,
                            data: js_sys::Uint8Array,
                            debug_types: Option<Vec<String>>,
                            debug_locations: Option<Vec<String>>| {
            let data = data.to_vec();
            #[cfg(not(debug_assertions))]
            let _ = (debug_types, debug_locations);
            _ = tx.start_send(SuspenseMessage {
                suspense_path: path,
                data,
                #[cfg(debug_assertions)]
                debug_types,
                #[cfg(debug_assertions)]
                debug_locations,
            });
        };
        let closure = wasm_bindgen::closure::Closure::new(closure);

        // EnterRoot(i) parks the cursor on under[i], so pass the mount
        // children. The JS cursor filters dx-injected hydration scripts before
        // exposing the root list.
        self.collect_initial_suspense(vdom.base_scope(), vdom);
        let roots = children_array(&self.root);
        self.start_hydration_at_scope(vdom.base_scope(), vdom, roots, true, false)?;

        dioxus_interpreter_js::minimal_bindings::register_rehydrate_chunk_for_streaming(&closure);
        closure.forget();

        Ok(rx)
    }
}
