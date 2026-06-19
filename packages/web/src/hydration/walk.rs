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
//! (`root_slots` / `static_children` / dynamic anchors, the same document-order
//! walk SSR uses in `dioxus_ssr`); mounted `ElementId`s come from the rendered
//! `MountedVNode`. No DOM reads happen here — the cursor performs them.

use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, MountedVNode, ScopeState, VNode, VirtualDom,
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
        for (root_idx, static_op, dynamic_anchor) in vnode.vnode().template.root_slots() {
            if let Some(anchor) = dynamic_anchor {
                for value_idx in vnode.vnode().dynamic_node_indices_for_anchor(anchor) {
                    self.emit_dynamic_node_at_level(vnode, value_idx, dom, cursor, state)?;
                }
            } else {
                let op = static_op.expect("template root slot is static or dynamic");
                let root_id = vnode.mounted_root(root_idx, dom);
                self.emit_template_node_at_level(vnode, op, root_id, dom, cursor, state)?;
            }
        }
        Ok(())
    }

    /// Emit an element's children, interleaving static children
    /// with dynamic node slots in document order (mirrors `dioxus_ssr`).
    fn emit_element_children_at_level<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        element_op: usize,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
        state: &mut LevelState,
    ) -> Result<(), RehydrationError> {
        let mut static_children = vnode.vnode().template.static_children(element_op);
        for slot in 0.. {
            for anchor in vnode
                .vnode()
                .dynamic_node_anchors_for_slot(element_op, slot)
            {
                for value_idx in vnode.vnode().dynamic_node_indices_for_anchor(anchor) {
                    self.emit_dynamic_node_at_level(vnode, value_idx, dom, cursor, state)?;
                }
            }
            let Some(op) = static_children.next() else {
                break;
            };
            self.emit_template_node_at_level(vnode, op, None, dom, cursor, state)?;
        }
        Ok(())
    }

    fn emit_template_node_at_level<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        op: usize,
        root_id: Option<ElementId>,
        dom: &'a VirtualDom,
        cursor: &mut HydrationCursor,
        state: &mut LevelState,
    ) -> Result<(), RehydrationError> {
        let template = vnode.vnode().template;
        if template.element_meta_at_op(op).is_some() {
            state.flush_text(cursor)?;
            state.advance(cursor);
            self.emit_element(vnode, op, root_id, cursor, dom)?;
            state.prev_consumed = 1;
        } else if let Some(text) = template.static_text_at_op(op) {
            state.push_text(utf16_len(text), root_id);
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
        op: usize,
        root_id: Option<ElementId>,
        cursor: &mut HydrationCursor,
        dom: &'a VirtualDom,
    ) -> Result<(), RehydrationError> {
        let (tag, _namespace) = vnode
            .vnode()
            .template
            .element_meta_at_op(op)
            .ok_or(HydrationMismatch)?;

        // Resolve the mounted ElementId (the dynamic attr id overrides the root
        // id when present) and collect dynamic listeners + onmounted events.
        let mut mounted_id = root_id;
        for anchor in vnode.vnode().dynamic_attr_anchors_for_element(op) {
            for value_idx in vnode.vnode().dynamic_attr_indices_for_anchor(anchor) {
                let attr_id = vnode
                    .mounted_dynamic_attribute(value_idx, dom)
                    .ok_or(VNodeNotInitialized)?;
                mounted_id = Some(attr_id);
            }
        }

        // Always map the element so the cursor can verify the tag and step past
        // parser-inserted wrappers. id == 0 means the element needs no node
        // binding but still occupies a positional slot.
        let id_arg = mounted_id.map(|i| i.raw() as u32).unwrap_or(0);
        cursor.map_element(tag, id_arg)?;

        for anchor in vnode.vnode().dynamic_attr_anchors_for_element(op) {
            for value_idx in vnode.vnode().dynamic_attr_indices_for_anchor(anchor) {
                let Some(attrs) = vnode.vnode().dynamic_values()[value_idx].as_attrs() else {
                    return Err(HydrationMismatch);
                };
                for attribute in attrs {
                    if matches!(attribute.value, AttributeValue::Listener(_)) {
                        if attribute.name == "onmounted" {
                            #[cfg(feature = "mounted")]
                            {
                                let attr_id = vnode
                                    .mounted_dynamic_attribute(value_idx, dom)
                                    .ok_or(VNodeNotInitialized)?;
                                self.send_mount_event(attr_id);
                            }
                        } else {
                            let event_name =
                                attribute.name.strip_prefix("on").unwrap_or(attribute.name);
                            let bubbles = dioxus_core_types::event_bubbles(event_name);
                            cursor.attach_listener(id_arg, event_name, bubbles);
                        }
                    }
                }
            }
        }

        // Descend only if the subtree contains dynamic content. Pure-static
        // subtrees match the server output by construction and need no walk —
        // `advance(1)` past the mapped element steps over them.
        if element_has_dynamic_content(vnode.vnode(), op) {
            cursor.begin_children();
            let mut state = LevelState::default();
            self.emit_element_children_at_level(vnode, op, dom, cursor, &mut state)?;
            state.finish(cursor)?;
            cursor.end_children();
        }

        Ok(())
    }
}

/// True if `op`'s element subtree contains any dynamic node child or a nested
/// element with dynamic attributes / dynamic content. Used to skip the walk
/// (and DOM reads) for purely static subtrees. Uses only public template API.
fn element_has_dynamic_content(vnode: &VNode, op: usize) -> bool {
    if vnode.dynamic_node_anchors_for_element(op).next().is_some() {
        return true;
    }
    for child_op in vnode.template.static_children(op) {
        if vnode.template.element_meta_at_op(child_op).is_some() {
            if vnode
                .dynamic_attr_anchors_for_element(child_op)
                .next()
                .is_some()
            {
                return true;
            }
            if element_has_dynamic_content(vnode, child_op) {
                return true;
            }
        }
    }
    false
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
