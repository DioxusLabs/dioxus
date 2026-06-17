//! The markerless hydration walk.
//!
//! SSR emits no hydration markers, so the client reconstructs the DOM shape by
//! walking the rebuilt VDOM and matching it against the real DOM with a
//! [`HydrationCursor`]. Each level of the VDOM is flattened into ordered
//! [`Leaf`]s (components and fragments are transparent); adjacent text leaves
//! are grouped into text-runs because the browser merges them into a single DOM
//! text node, which the cursor addresses with `splitText` offsets.
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
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        self.collect_suspense_only(scope, dom);

        let mut leaves: Vec<Leaf<'a>> = Vec::new();
        let root = scope.try_mounted_root_node().ok_or(VNodeNotInitialized)?;
        self.collect_vnode_root_leaves(root, dom, &mut leaves)?;
        self.emit_leaves(&leaves, cursor, dom, to_mount)
    }

    /// Flatten a VNode's template roots into leaves at the current DOM level.
    fn collect_vnode_root_leaves<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        dom: &'a VirtualDom,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        let roots: Vec<_> = vnode.vnode().template.root_slots().collect();
        for (root_idx, static_op, dynamic_anchor) in roots {
            if let Some(anchor) = dynamic_anchor {
                for value_idx in vnode.vnode().dynamic_node_indices_for_anchor(anchor) {
                    self.collect_dynamic_node_leaves(vnode, value_idx, dom, out)?;
                }
            } else {
                let op = static_op.expect("template root slot is static or dynamic");
                let root_id = vnode.mounted_root(root_idx, dom);
                self.collect_template_node_leaves(vnode, op, root_id, out)?;
            }
        }
        Ok(())
    }

    /// Flatten an element's children into leaves, interleaving static children
    /// with dynamic node slots in document order (mirrors `dioxus_ssr`).
    fn collect_element_child_leaves<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        element_op: usize,
        dom: &'a VirtualDom,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        let static_children: Vec<usize> =
            vnode.vnode().template.static_children(element_op).collect();
        for slot in 0..=static_children.len() {
            let anchors: Vec<_> = vnode
                .vnode()
                .dynamic_node_anchors_for_slot(element_op, slot)
                .collect();
            for anchor in anchors {
                for value_idx in vnode.vnode().dynamic_node_indices_for_anchor(anchor) {
                    self.collect_dynamic_node_leaves(vnode, value_idx, dom, out)?;
                }
            }
            if let Some(&op) = static_children.get(slot) {
                self.collect_template_node_leaves(vnode, op, None, out)?;
            }
        }
        Ok(())
    }

    fn collect_template_node_leaves<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        op: usize,
        root_id: Option<ElementId>,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        let template = vnode.vnode().template;
        if template.element_meta_at_op(op).is_some() {
            out.push(Leaf::Element { vnode, op, root_id });
        } else if let Some(text) = template.static_text_at_op(op) {
            out.push(Leaf::StaticText { text, id: root_id });
        }
        Ok(())
    }

    fn collect_dynamic_node_leaves<'a>(
        &mut self,
        vnode: MountedVNode<'a>,
        value_idx: usize,
        dom: &'a VirtualDom,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        match vnode.vnode().dynamic_values[value_idx]
            .as_node()
            .expect("hydration node slot must point at a dynamic node")
        {
            DynamicNode::Text(text) => {
                let id = vnode
                    .mounted_dynamic_node(value_idx, dom)
                    .ok_or(VNodeNotInitialized)?;
                out.push(Leaf::DynamicText {
                    value: &text.value,
                    id,
                });
            }
            DynamicNode::Component(comp) => {
                let scope = comp
                    .mounted_scope(value_idx, vnode, dom)
                    .ok_or(VNodeNotInitialized)?;
                let child = scope.try_mounted_root_node().ok_or(VNodeNotInitialized)?;
                self.collect_vnode_root_leaves(child, dom, out)?;
            }
            DynamicNode::Fragment(fragment) => {
                let mounted_children = vnode.mounted_fragment_children(value_idx, dom);
                if mounted_children.len() != fragment.len() {
                    return Err(VNodeNotInitialized);
                }

                for sub_vnode in mounted_children {
                    self.collect_vnode_root_leaves(sub_vnode, dom, out)?;
                }
            }
        }
        Ok(())
    }

    /// Drive the cursor over the leaves at one DOM level, grouping adjacent text
    /// and embedded-placeholder leaves into text-runs (the browser merges them
    /// into one DOM text node).
    fn emit_leaves<'a>(
        &mut self,
        leaves: &[Leaf<'a>],
        cursor: &mut HydrationCursor,
        dom: &'a VirtualDom,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        let steps = group_steps(leaves);

        let mut prev_consumed: u32 = 0;
        for (idx, step) in steps.iter().enumerate() {
            if prev_consumed > 0 {
                cursor.advance(prev_consumed);
            }
            prev_consumed = match step {
                EmitStep::Element(leaf) => {
                    self.emit_element(leaf, cursor, dom, to_mount)?;
                    1
                }
                EmitStep::TextRun(run) => {
                    let split_tail = next_consuming_is_text_run(&steps, idx);
                    emit_text_run(run, cursor, split_tail)?
                }
            };
        }
        Ok(())
    }

    fn emit_element<'a>(
        &mut self,
        leaf: &Leaf<'a>,
        cursor: &mut HydrationCursor,
        dom: &'a VirtualDom,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        let &Leaf::Element { vnode, op, root_id } = leaf else {
            unreachable!("emit_element only accepts element leaves");
        };
        let (tag, _namespace) = vnode
            .vnode()
            .template
            .element_meta_at_op(op)
            .expect("element leaf wraps an element op");

        // Resolve the mounted ElementId (the dynamic attr id overrides the root
        // id when present) and collect dynamic listeners + onmounted events.
        let mut mounted_id = root_id;
        let mut listeners: Vec<(&'static str, bool)> = Vec::new();
        for anchor in vnode.vnode().dynamic_attr_anchors_for_element(op) {
            for value_idx in vnode.vnode().dynamic_attr_indices_for_anchor(anchor) {
                let attr_id = vnode
                    .mounted_dynamic_attribute(value_idx, dom)
                    .ok_or(VNodeNotInitialized)?;
                mounted_id = Some(attr_id);
                let attrs = vnode.vnode().dynamic_values[value_idx]
                    .as_attrs()
                    .expect("hydration attr slot must point at dynamic attributes");
                for attribute in attrs {
                    if matches!(attribute.value, AttributeValue::Listener(_)) {
                        if attribute.name == "onmounted" {
                            to_mount.push(attr_id);
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

        // Always map the element so the cursor can verify the tag and step past
        // parser-inserted wrappers. id == 0 means the element needs no node
        // binding but still occupies a positional slot.
        let id_arg = mounted_id.map(|i| i.raw() as u32).unwrap_or(0);
        let element = cursor.map_element(tag, id_arg)?;

        for (name, bubbles) in listeners {
            cursor.attach_listener(&element, id_arg, name, bubbles);
        }

        // Descend only if the subtree contains dynamic content. Pure-static
        // subtrees match the server output by construction and need no walk —
        // `advance(1)` past the mapped element steps over them.
        if element_has_dynamic_content(vnode.vnode(), op) {
            cursor.begin_children();
            let mut child_leaves: Vec<Leaf<'a>> = Vec::new();
            self.collect_element_child_leaves(vnode, op, dom, &mut child_leaves)?;
            self.emit_leaves(&child_leaves, cursor, dom, to_mount)?;
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
    for child_op in vnode.template.static_children(op).collect::<Vec<_>>() {
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

/// A flattened item at one DOM level. Components and fragments are transparent —
/// expanded into their constituent leaves.
#[derive(Clone, Copy)]
pub(super) enum Leaf<'a> {
    /// Static literal text from the template. `id` is `Some` only when this text
    /// is a *root* of some VNode.
    StaticText {
        text: &'a str,
        id: Option<ElementId>,
    },
    /// Runtime text from a `DynamicNode::Text`.
    DynamicText { value: &'a str, id: ElementId },
    /// A static template element plus the owning VNode (for resolving dynamic
    /// attribute slots and children) and its op index in the template tape.
    Element {
        vnode: MountedVNode<'a>,
        op: usize,
        root_id: Option<ElementId>,
    },
}

impl Leaf<'_> {
    fn is_text(&self) -> bool {
        matches!(self, Leaf::StaticText { .. } | Leaf::DynamicText { .. })
    }
}

/// One emit-level step. Text runs group adjacent text leaves so they can be
/// addressed against the browser-merged DOM text node with `splitText` offsets.
enum EmitStep<'a, 'b> {
    Element(&'b Leaf<'a>),
    TextRun(&'b [Leaf<'a>]),
}

impl EmitStep<'_, '_> {
    fn consumes_dom(&self) -> bool {
        match self {
            EmitStep::TextRun(leaves) => leaves.iter().any(leaf_text_is_non_empty),
            EmitStep::Element(_) => true,
        }
    }

    fn is_text_run(&self) -> bool {
        matches!(self, EmitStep::TextRun(_))
    }
}

/// Group leaves into emit steps. A text leaf greedy-extends across consecutive
/// text leaves (the browser merges all text contributions into one DOM node).
fn group_steps<'a, 'b>(leaves: &'b [Leaf<'a>]) -> Vec<EmitStep<'a, 'b>> {
    let mut steps = Vec::new();
    let mut i = 0;
    while i < leaves.len() {
        if leaves[i].is_text() {
            let mut end = i + 1;
            while end < leaves.len() && leaves[end].is_text() {
                end += 1;
            }
            steps.push(EmitStep::TextRun(&leaves[i..end]));
            i = end;
        } else {
            steps.push(EmitStep::Element(&leaves[i]));
            i += 1;
        }
    }
    steps
}

/// True when the next *consuming* step after `idx` is a text run, meaning our
/// last non-empty text contribution needs `split_after` so the cursor advances
/// past it (instead of parking on it for the caller to advance 1).
fn next_consuming_is_text_run(steps: &[EmitStep<'_, '_>], idx: usize) -> bool {
    let mut next = idx + 1;
    while let Some(step) = steps.get(next) {
        if step.is_text_run() {
            return true;
        }
        if step.consumes_dom() {
            return false;
        }
        next += 1;
    }
    false
}

/// Drive the cursor over one text run. All contributions share a single
/// browser-merged DOM text node (or zero, if every contribution is empty).
///
/// Returns the number of DOM siblings this run consumed (0 if all-empty or
/// `split_tail`, else 1).
fn emit_text_run(
    run: &[Leaf<'_>],
    cursor: &mut HydrationCursor,
    split_tail: bool,
) -> Result<u32, RehydrationError> {
    let last_nonempty = run.iter().rposition(leaf_text_is_non_empty);

    for (i, leaf) in run.iter().enumerate() {
        match *leaf {
            Leaf::StaticText { text, id } => {
                emit_text_leaf(cursor, utf16_len(text), id, i, last_nonempty, split_tail)?;
            }
            Leaf::DynamicText { value, id } => {
                emit_text_leaf(
                    cursor,
                    utf16_len(value),
                    Some(id),
                    i,
                    last_nonempty,
                    split_tail,
                )?;
            }
            Leaf::Element { .. } => unreachable!("non-text leaf in text run"),
        }
    }

    if last_nonempty.is_none() || split_tail {
        Ok(0)
    } else {
        Ok(1)
    }
}

fn emit_text_leaf(
    cursor: &mut HydrationCursor,
    len: u32,
    id: Option<ElementId>,
    i: usize,
    last_nonempty: Option<usize>,
    split_tail: bool,
) -> Result<(), RehydrationError> {
    if len == 0 {
        let Some(id) = id else { return Ok(()) };
        // All-empty runs park every sentinel before the cursor (which is on
        // whatever follows the run). Otherwise position relative to the last
        // non-empty contribution: before goes _before_ the cursor, after goes
        // _after_ it.
        let before_cursor = match last_nonempty {
            None => true,
            Some(last) => i < last,
        };
        if before_cursor {
            cursor.synth_text(id.raw() as u32)?;
        } else {
            cursor.synth_text_after(id.raw() as u32)?;
        }
    } else {
        let id_arg = id.map(|i| i.raw() as u32).unwrap_or(0);
        let split_after = matches!(last_nonempty, Some(last) if i < last)
            || (split_tail && Some(i) == last_nonempty);
        cursor.text_contrib(len, id_arg, split_after)?;
    }
    Ok(())
}

fn leaf_text_is_non_empty(leaf: &Leaf<'_>) -> bool {
    match leaf {
        Leaf::StaticText { text, .. } => utf16_len(text) > 0,
        Leaf::DynamicText { value, .. } => utf16_len(value) > 0,
        _ => false,
    }
}

/// UTF-16 length, matching `Text.length` / `splitText` offsets in JS.
pub(super) fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}
