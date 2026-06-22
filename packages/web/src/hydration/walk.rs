//! The markerless hydration walk.
//!
//! SSR emits no hydration markers, so the client reconstructs the DOM shape by
//! walking the rebuilt VDOM and matching it against the real DOM with a
//! [`HydrationCursor`]. Each level is walked in document order while buffering
//! adjacent text contributions into text-runs because the browser merges them
//! into a single DOM text node, which the cursor addresses with `splitText`
//! offsets. Components and fragments are transparent and emit into the current
//! level.
//!
//! The template structure comes from the flat op-tape [`dioxus_core::Template`]
//! (`VNode::children` / element children / dynamic anchors, the same document-order
//! walk SSR uses in `dioxus_ssr`); mounted `ElementId`s come from the rendered
//! `MountedVNode`. No DOM reads happen here — the cursor performs them.

use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, MountedVNode, ScopeState, StaticElement, VNodeChild,
    VirtualDom,
};

use crate::dom::WebsysDom;

use super::cursor::HydrationCursor;
use super::{RehydrationError, RehydrationError::*};

impl WebsysDom {
    /// Top-level hydration emitter. Walks the rebuilt VDOM for `scope` and drives
    /// the [`HydrationCursor`] over the matching server-rendered DOM.
    pub(super) fn emit_scope<'a>(
        &mut self,
        scope: &'a ScopeState,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
    ) -> Result<(), RehydrationError> {
        self.collect_suspense_only(scope, dom);

        let root = scope.try_mounted_root_node().ok_or(VNodeNotInitialized)?;
        let mut state = LevelState::default();
        self.emit_vnode_roots_at_level(root, dom, cursor, &mut state)?;
        state.finish(cursor)
    }

    /// Emit a VNode's template roots at the current DOM level.
    fn emit_vnode_roots_at_level<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
        state: &mut LevelState,
    ) -> Result<(), RehydrationError> {
        for child in vnode.vnode().children() {
            self.emit_child_at_level(vnode, child, dom, cursor, state)?;
        }
        Ok(())
    }

    /// Emit an element's children, interleaving static children
    /// with dynamic node slots in document order (mirrors `dioxus_ssr`).
    fn emit_element_children_at_level<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        element: StaticElement<'a>,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
        state: &mut LevelState,
    ) -> Result<(), RehydrationError> {
        for child in element.children() {
            self.emit_child_at_level(vnode, child, dom, cursor, state)?;
        }
        Ok(())
    }

    fn emit_child_at_level<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        child: VNodeChild<'a>,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
        state: &mut LevelState,
    ) -> Result<(), RehydrationError> {
        match child {
            VNodeChild::Dynamic(group) => {
                for value_idx in group.ids() {
                    self.emit_dynamic_node_at_level(vnode, value_idx, dom, cursor, state)?;
                }
            }
            VNodeChild::Element(element) => {
                let root_id = element
                    .root_position()
                    .and_then(|root_position| vnode.mounted_root(root_position, dom));
                state.flush_text(cursor)?;
                state.advance(cursor);
                self.emit_element(vnode, element, root_id, cursor, dom)?;
                state.prev_consumed = 1;
            }
            VNodeChild::Text(text) => {
                let root_id = text
                    .root_position()
                    .and_then(|root_position| vnode.mounted_root(root_position, dom));
                state.push_text(utf16_len(text.text()), root_id);
            }
        }
        Ok(())
    }

    fn emit_dynamic_node_at_level<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        value_idx: usize,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
        state: &mut LevelState,
    ) -> Result<(), RehydrationError> {
        let Some(node) = vnode.vnode().dynamic_values()[value_idx].as_node() else {
            return Err(HydrationMismatch);
        };

        match node {
            DynamicNode::Text(text) => {
                let id = vnode
                    .mounted_dynamic_node(value_idx, dom)
                    .ok_or(VNodeNotInitialized)?;
                state.push_text(utf16_len(&text.value), Some(id));
            }
            DynamicNode::Component(comp) => {
                let scope = comp
                    .mounted_scope(value_idx, vnode, dom)
                    .ok_or(VNodeNotInitialized)?;
                let child = scope.try_mounted_root_node().ok_or(VNodeNotInitialized)?;
                self.emit_vnode_roots_at_level(child, dom, cursor, state)?;
            }
            DynamicNode::Fragment(fragment) => {
                let mounted_children = vnode.mounted_fragment_children(value_idx, dom);
                if mounted_children.len() != fragment.len() {
                    return Err(VNodeNotInitialized);
                }

                for sub_vnode in mounted_children {
                    self.emit_vnode_roots_at_level(sub_vnode, dom, cursor, state)?;
                }
            }
        }
        Ok(())
    }

    fn emit_element<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        element: StaticElement<'a>,
        root_id: Option<ElementId>,
        cursor: &mut HydrationCursor,
        dom: &'a VirtualDom,
    ) -> Result<(), RehydrationError> {
        let tag = element.tag();

        // Resolve the mounted ElementId (an anchor id overrides the root id when present) and
        // collect dynamic listeners + onmounted events.
        let mut mounted_id = root_id;
        let mut listeners = Vec::new();
        #[cfg(feature = "mounted")]
        let mut mounted_events = Vec::new();
        self.collect_dynamic_attrs_for_element(
            vnode,
            element,
            dom,
            &mut mounted_id,
            &mut listeners,
            #[cfg(feature = "mounted")]
            &mut mounted_events,
        )?;

        // Always map the element so the cursor can verify the tag and step past
        // parser-inserted wrappers. id == 0 means the element needs no node
        // binding but still occupies a positional slot.
        let id_arg = mounted_id.map(|i| i.raw() as u32).unwrap_or(0);
        cursor.map_element(tag, id_arg)?;

        #[cfg(feature = "mounted")]
        for anchor_id in mounted_events {
            self.send_mount_event(anchor_id);
        }
        for (event_name, bubbles) in listeners {
            cursor.attach_listener(id_arg, event_name, bubbles);
        }

        if element.has_children() {
            cursor.begin_children();
            let mut state = LevelState::default();
            self.emit_element_children_at_level(vnode, element, dom, cursor, &mut state)?;
            state.finish(cursor)?;
            cursor.end_children();
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_dynamic_attrs_for_element<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        element: StaticElement<'a>,
        dom: &'a VirtualDom,
        mounted_id: &mut Option<ElementId>,
        listeners: &mut Vec<(&'static str, bool)>,
        #[cfg(feature = "mounted")] mounted_events: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        for group in element.dynamic_attributes() {
            let anchor_id = vnode
                .mounted_anchor_node_by_index(group.anchor_index(), dom)
                .ok_or(VNodeNotInitialized)?;
            *mounted_id = Some(anchor_id);
            for value_idx in group.ids() {
                let Some(attrs) = vnode.vnode().dynamic_values()[value_idx].as_attrs() else {
                    return Err(HydrationMismatch);
                };
                for attribute in attrs {
                    if matches!(attribute.value, AttributeValue::Listener(_)) {
                        if attribute.name == "onmounted" {
                            #[cfg(feature = "mounted")]
                            mounted_events.push(anchor_id);
                        } else {
                            let event_name =
                                attribute.name.strip_prefix("on").unwrap_or(attribute.name);
                            let bubbles = dioxus_core_types::event_bubbles(event_name);
                            listeners.push((event_name, bubbles));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct TextContribution {
    len: u32,
    id: Option<ElementId>,
}

#[derive(Default)]
struct LevelState {
    pending_text: Vec<TextContribution>,
    prev_consumed: u32,
}

impl LevelState {
    fn push_text(&mut self, len: u32, id: Option<ElementId>) {
        self.pending_text.push(TextContribution { len, id });
    }

    fn flush_text(&mut self, cursor: &mut HydrationCursor) -> Result<(), RehydrationError> {
        if self.pending_text.is_empty() {
            return Ok(());
        }

        self.advance(cursor);
        self.prev_consumed = emit_text_run(&self.pending_text, cursor)?;
        self.pending_text.clear();
        Ok(())
    }

    fn finish(&mut self, cursor: &mut HydrationCursor) -> Result<(), RehydrationError> {
        self.flush_text(cursor)
    }

    fn advance(&mut self, cursor: &mut HydrationCursor) {
        if self.prev_consumed > 0 {
            cursor.advance(self.prev_consumed);
            self.prev_consumed = 0;
        }
    }
}

/// Drive the cursor over one text run. All contributions share a single
/// browser-merged DOM text node (or zero, if every contribution is empty).
///
/// Returns the number of DOM siblings this run consumed (0 if all-empty, else 1).
fn emit_text_run(
    run: &[TextContribution],
    cursor: &mut HydrationCursor,
) -> Result<u32, RehydrationError> {
    let last_nonempty = run.iter().rposition(|contribution| contribution.len > 0);

    for (i, contribution) in run.iter().enumerate() {
        emit_text_leaf(cursor, *contribution, i, last_nonempty)?;
    }

    if last_nonempty.is_none() {
        Ok(0)
    } else {
        Ok(1)
    }
}

fn emit_text_leaf(
    cursor: &mut HydrationCursor,
    contribution: TextContribution,
    i: usize,
    last_nonempty: Option<usize>,
) -> Result<(), RehydrationError> {
    if contribution.len == 0 {
        let Some(id) = contribution.id else {
            return Ok(());
        };
        // All-empty runs insert before the cursor. Otherwise, empty sentinels
        // after the last real text contribution are inserted after it.
        cursor.synth(id.raw() as u32, last_nonempty.is_some_and(|last| i >= last))?;
    } else {
        let id_arg = contribution.id.map(|i| i.raw() as u32).unwrap_or(0);
        let split_after = matches!(last_nonempty, Some(last) if i < last);
        cursor.text_contrib(contribution.len, id_arg, split_after)?;
    }
    Ok(())
}

/// UTF-16 length, matching `Text.length` / `splitText` offsets in JS.
pub(super) fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}
