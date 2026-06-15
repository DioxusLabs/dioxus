//! When hydrating streaming components:
//! 1. Hydrate the already-rendered DOM on the outside
//! 2. As we render the virtual dom initially, keep track of the server ids of the suspense boundaries
//! 3. Register a callback for dx_hydrate(id, data) that takes some new data, reruns the suspense boundary with that new data and then rehydrates the node

use crate::dom::WebsysDom;
use dioxus_core::{ScopeState, SuspenseBoundaryProps, VirtualDom};
use dioxus_fullstack_core::HydrationContext;
use dioxus_interpreter_js::hydration_bindings::{
    HydrationChannel, claim_hydration_virtual_root, install_hydration_state,
    push_hydration_virtual_root,
};
use futures_channel::mpsc::UnboundedReceiver;
use wasm_bindgen::JsCast;

use super::SuspenseMessage;
use super::suspense::{first_dynamic_root_element_id, path_to_resolved_suspense_id};

#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum RehydrationError {
    /// The client tried to rehydrate a vnode before the dom was built
    VNodeNotInitialized,
    /// The client tried to rehydrate a suspense boundary that was not mounted on the server
    SuspenseHydrationIdNotFound,
    /// The client tried to rehydrate a dom id that was not found on the server
    ElementNotFound,
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

        // Collect the new nodes. `resolve_suspense` pushes them after it
        // pushes the placeholder target so replacement stays stack-only.
        let mut current_child = resolved_suspense_element.first_child();
        let mut children = Vec::new();
        while let Some(node) = current_child {
            children.push(node.clone());
            current_child = node.next_sibling();
        }

        // Empty errored chunks use an anchor ref as the
        // `replace_with(loading_id, 1)` item. `applyChunk` fills in
        // `parent`/`before` from the loading slot; hydration later binds the
        // new scope's placeholder ElementId to that same anchor.
        let empty_bootstrap = children.is_empty();
        let mut empty_bootstrap_anchor = None;

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
            SuspenseBoundaryProps::resolve_suspense(
                id,
                dom,
                self,
                |to| {
                    // Resolve the VDOM bookkeeping without writing duplicate DOM nodes.
                    to.skip_mutations = true;
                },
                |to| {
                    if empty_bootstrap {
                        empty_bootstrap_anchor =
                            Some(push_hydration_virtual_root(to.interpreter.base()));
                        1
                    } else {
                        for node in &children {
                            to.interpreter.base().push_root(node.clone());
                        }
                        children.len()
                    }
                },
            );
            self.skip_mutations = false;
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
            // Empty-chunk path: the anchor pushed earlier already has
            // `parent`/`before` set from `replace_with`. Bind it to the
            // resolved scope's first empty dynamic-root ElementId so
            // subsequent anchor ops resolve through it, then walk the scope
            // to record nested suspense ids.
            if let (Some(claim_id), Some(anchor)) = (
                first_dynamic_root_element_id(root_scope, dom),
                empty_bootstrap_anchor.as_ref(),
            ) {
                claim_hydration_virtual_root(
                    self.interpreter.base(),
                    claim_id.raw() as u32,
                    anchor,
                );
            }
            self.collect_suspense_only(root_scope, dom);
        } else {
            self.start_hydration_at_scope(root_scope, dom, children)?;
        }

        Ok(())
    }

    fn start_hydration_at_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        under: Vec<web_sys::Node>,
    ) -> Result<(), RehydrationError> {
        let under_is_empty = under.is_empty();
        let under = if under_is_empty {
            vec![self.root.clone()]
        } else {
            under
        };
        let mut channel = HydrationChannel::default();
        let mut to_mount = Vec::new();

        if under_is_empty {
            // No real root nodes were emitted by SSR. Hydrate under the mount
            // element so zero-DOM root ids still get anchors with a concrete
            // parent.
            channel.hy_enter_root(0);
            channel.hy_begin_children();
            self.emit_scope(scope, dom, &mut channel, &mut to_mount)?;
            channel.hy_end_children();
        } else {
            // Park the cursor on the first root; subsequent roots are stepped via
            // `Advance(1)` ops emitted by the VDOM walker.
            channel.hy_enter_root(0);
            self.emit_scope(scope, dom, &mut channel, &mut to_mount)?;
        }

        // Bind the DOM roots to the live mutation interpreter before flushing
        // the queued hydration ops.
        install_hydration_state(channel.js_channel(), self.interpreter.base(), under);
        channel.flush();

        #[cfg(feature = "mounted")]
        for id in to_mount {
            self.send_mount_event(id);
        }

        Ok(())
    }

    pub fn rehydrate(
        &mut self,
        vdom: &VirtualDom,
    ) -> Result<UnboundedReceiver<SuspenseMessage>, RehydrationError> {
        let (mut tx, rx) = futures_channel::mpsc::unbounded();
        let closure =
            move |path: Vec<u32>,
                  data: js_sys::Uint8Array,
                  #[allow(unused)] debug_types: Option<Vec<String>>,
                  #[allow(unused)] debug_locations: Option<Vec<String>>| {
                let data = data.to_vec();
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
        dioxus_interpreter_js::minimal_bindings::register_rehydrate_chunk_for_streaming_debug(
            &closure,
        );
        closure.forget();

        // EnterRoot(i) parks the cursor on under[i], so pass the rendered app
        // roots. The dx build injects `<script>` tags before/after the app for
        // hydration-data plumbing; those are not part of the rendered app.
        let mut roots: Vec<web_sys::Node> = Vec::new();
        let mut current = self.root.first_child();
        while let Some(node) = current {
            current = node.next_sibling();
            let is_injected_script = node
                .dyn_ref::<web_sys::Element>()
                .is_some_and(|el| el.tag_name().eq_ignore_ascii_case("script"));
            if !is_injected_script {
                roots.push(node);
            }
        }
        self.start_hydration_at_scope(vdom.base_scope(), vdom, roots)?;

        Ok(rx)
    }
}
