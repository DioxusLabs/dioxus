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
use super::suspense::{first_dynamic_root_element_id, path_to_resolved_suspense_id};

fn children_array(parent: &web_sys::Node) -> js_sys::Array {
    js_sys::Array::from(parent.child_nodes().unchecked_ref())
}

fn node_array(node: &web_sys::Node) -> js_sys::Array {
    let array = js_sys::Array::new();
    array.push(node.unchecked_ref());
    array
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[non_exhaustive]
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
            #[cfg(debug_assertions)]
            tracing::error!("Rehydration failed. {:?}", err);
            #[cfg(not(debug_assertions))]
            {
                let _ = err;
                tracing::error!("Rehydration failed.");
            }
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

        // Empty errored chunks use a real empty text node as the
        // `replace_with(loading_id, 1)` item; the replacement positions it at
        // the loading slot. Hydration later binds the new scope's placeholder
        // ElementId to that same node.
        let empty_bootstrap = children_len == 0;
        let mut empty_bootstrap_node = None;

        #[cfg(not(debug_assertions))]
        let debug_types = None;
        #[cfg(not(debug_assertions))]
        let debug_locations = None;

        let server_data = HydrationContext::from_serialized(&data, debug_types, debug_locations);
        // If the server serialized an error into the suspense boundary, throw it on the client so that it bubbles up to the nearest error boundary
        if let Some(error) = server_data.error_entry().get().ok().flatten() {
            dom.in_runtime(|| dom.runtime().throw_error(id, error));
        }
        server_data.in_context(|| {
            // rerun the scope with the new data
            SuspenseBoundaryProps::resolve_suspense(id, dom, self, |to| {
                if empty_bootstrap {
                    let node: web_sys::Node = document.create_text_node("").unchecked_into();
                    to.interpreter.base().push_root(node.clone());
                    empty_bootstrap_node = Some(node);
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

        if empty_bootstrap {
            // Empty-chunk path: the empty text node pushed earlier was placed
            // at the loading slot by `replace_with`. Bind it to the resolved
            // scope's first empty dynamic-root ElementId so later mutations
            // target it, then walk the scope to record nested suspense ids.
            if let (Some(claim_id), Some(node)) = (
                first_dynamic_root_element_id(root_scope, dom),
                empty_bootstrap_node.as_ref(),
            ) {
                self.interpreter.base().push_root(node.clone());
                self.interpreter.pop_id(claim_id.raw() as u32);
                self.flush_edits();
            }
            self.collect_suspense_only(root_scope, dom);
        } else {
            self.start_hydration_at_scope(root_scope, dom, children, false)?;
        }

        Ok(())
    }

    fn start_hydration_at_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        under: js_sys::Array,
        filter_scripts: bool,
    ) -> Result<(), RehydrationError> {
        let mut cursor = HydrationCursor::new(
            self.interpreter.base(),
            self.root.clone(),
            under,
            filter_scripts,
        );
        let under_is_empty = cursor.root_count() == 0;

        if under_is_empty {
            // No real root nodes were emitted by SSR. Hydrate under the mount
            // element so zero-DOM root ids still get anchors with a concrete
            // parent.
            cursor = HydrationCursor::new(
                self.interpreter.base(),
                self.root.clone(),
                node_array(&self.root),
                false,
            );
            cursor.enter_root(0);
            cursor.begin_children();
            self.emit_scope(scope, dom, &mut cursor)?;
            cursor.end_children();
        } else {
            // Park the cursor on the first root; subsequent roots are stepped via
            // `advance(1)` as the VDOM walker descends siblings.
            cursor.enter_root(0);
            self.emit_scope(scope, dom, &mut cursor)?;
        }

        Ok(())
    }

    pub fn rehydrate(
        &mut self,
        vdom: &VirtualDom,
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
        dioxus_interpreter_js::minimal_bindings::register_rehydrate_chunk_for_streaming(&closure);
        closure.forget();

        // EnterRoot(i) parks the cursor on under[i], so pass the mount
        // children. The JS cursor filters dx-injected hydration scripts before
        // exposing the root list.
        let roots = children_array(&self.root);
        self.start_hydration_at_scope(vdom.base_scope(), vdom, roots, true)?;

        Ok(rx)
    }
}
