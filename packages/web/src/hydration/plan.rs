use crate::dom::WebsysDom;
use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, ScopeState, TemplateAttribute, TemplateNode, VNode,
    VirtualDom,
};
use dioxus_interpreter_js::hydration_bindings::HydrationChannel;

use super::{RehydrationError, RehydrationError::*};

impl WebsysDom {
    /// Top-level hydration emitter. Walks the rebuilt VDOM for `scope` and
    /// emits declarative `HydrationChannel` ops describing the expected DOM
    /// shape. Performs no DOM reads — element matching and transparent-
    /// wrapper tolerance happen on the JS side during op execution.
    pub(super) fn emit_scope(
        &mut self,
        scope: &ScopeState,
        dom: &VirtualDom,
        channel: &mut HydrationChannel,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        self.collect_suspense_only(scope, dom);

        let mut leaves: Vec<Leaf<'_>> = Vec::new();
        self.collect_vnode_root_leaves(scope.root_node(), dom, &mut leaves)?;
        self.emit_leaves(&leaves, channel, dom, to_mount)
    }

    /// Flatten a VNode's roots into leaves at the current DOM level.
    fn collect_vnode_root_leaves<'a>(
        &mut self,
        vnode: &'a VNode,
        dom: &'a VirtualDom,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        for (i, root) in vnode.template.roots().iter().enumerate() {
            let root_id = vnode.mounted_root(i, dom);
            self.collect_template_node_leaves(vnode, root, root_id, dom, out)?;
        }
        Ok(())
    }

    fn collect_template_node_leaves<'a>(
        &mut self,
        vnode: &'a VNode,
        node: &'a TemplateNode,
        root_id: Option<ElementId>,
        dom: &'a VirtualDom,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        match node {
            TemplateNode::Element { .. } => {
                out.push(Leaf::Element {
                    vnode,
                    node,
                    root_id,
                });
            }
            TemplateNode::Text { text } => {
                out.push(Leaf::StaticText { text, id: root_id });
            }
            TemplateNode::Dynamic { id } => {
                self.collect_dynamic_node_leaves(vnode, *id, dom, out)?;
            }
        }
        Ok(())
    }

    fn collect_dynamic_node_leaves<'a>(
        &mut self,
        vnode: &'a VNode,
        dyn_idx: usize,
        dom: &'a VirtualDom,
        out: &mut Vec<Leaf<'a>>,
    ) -> Result<(), RehydrationError> {
        match &vnode.dynamic_nodes[dyn_idx] {
            DynamicNode::Text(text) => {
                let id = vnode
                    .mounted_dynamic_node(dyn_idx, dom)
                    .ok_or(VNodeNotInitialized)?;
                out.push(Leaf::DynamicText {
                    value: &text.value,
                    id,
                });
            }
            DynamicNode::Component(comp) => {
                let scope = comp
                    .mounted_scope(dyn_idx, vnode, dom)
                    .ok_or(VNodeNotInitialized)?;
                self.collect_vnode_root_leaves(scope.root_node(), dom, out)?;
            }
            DynamicNode::Fragment(fragment) => {
                for sub_vnode in fragment {
                    self.collect_vnode_root_leaves(sub_vnode, dom, out)?;
                }
            }
        }
        Ok(())
    }

    /// Emit ops for the leaves at one DOM level, grouping adjacent text and
    /// embedded-placeholder leaves into text-runs (the browser merges them
    /// into one DOM text node).
    pub(super) fn emit_leaves<'a>(
        &mut self,
        leaves: &[Leaf<'a>],
        channel: &mut HydrationChannel,
        dom: &VirtualDom,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        let steps = group_steps(leaves);

        let mut prev_consumed: u32 = 0;
        for (idx, step) in steps.iter().enumerate() {
            if prev_consumed > 0 {
                channel.hy_advance(prev_consumed);
            }
            prev_consumed = match step {
                EmitStep::Element(leaf) => {
                    self.emit_element(*leaf, channel, dom, to_mount)?;
                    1
                }
                EmitStep::TextRun(run) => {
                    let split_tail = next_consuming_is_text_run(&steps, idx);
                    emit_text_run(run, channel, split_tail)
                }
            };
        }
        Ok(())
    }

    fn emit_element<'a>(
        &mut self,
        leaf: Leaf<'a>,
        channel: &mut HydrationChannel,
        dom: &VirtualDom,
        to_mount: &mut Vec<ElementId>,
    ) -> Result<(), RehydrationError> {
        let Leaf::Element {
            vnode,
            node,
            root_id,
        } = leaf
        else {
            unreachable!("emit_element only accepts element leaves");
        };
        let TemplateNode::Element {
            tag,
            attrs,
            children,
            ..
        } = node
        else {
            unreachable!("Leaf::Element wraps a TemplateNode::Element");
        };

        // Resolve mounted ElementId (root_id is overridden by the dynamic attr
        // id when present) and collect dynamic listeners + onmounted events.
        let mut mounted_id = root_id;
        let mut listeners: Vec<(&'static str, bool)> = Vec::new();
        for attr in *attrs {
            if let TemplateAttribute::Dynamic { id } = attr {
                let attr_id = vnode
                    .mounted_dynamic_attribute(*id, dom)
                    .ok_or(VNodeNotInitialized)?;
                mounted_id = Some(attr_id);
                for attribute in &*vnode.dynamic_attrs[*id] {
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

        // Always emit `hy_map_element` so JS can verify the tag and handle
        // parser-inserted wrappers (e.g. `<tbody>`). id == 0 indicates the
        // element doesn't need a mapping but still needs a positional anchor.
        let id_arg = mounted_id.map(|i| i.raw() as u32).unwrap_or(0);
        channel.hy_map_element(tag, id_arg);

        for (name, bubbles) in listeners {
            channel.hy_attach_listener(name, if bubbles { 1 } else { 0 });
        }

        // Descend only if the subtree contains any dynamic content. Pure-
        // static subtrees need no hydration walk — `hy_advance(1)` past the
        // mapped element steps over them at the parent level.
        if children.iter().any(template_node_has_dynamic_content) {
            let mut child_leaves: Vec<Leaf<'_>> = Vec::new();
            for child in *children {
                self.collect_template_node_leaves(vnode, child, None, dom, &mut child_leaves)?;
            }
            channel.hy_begin_children();
            self.emit_leaves(&child_leaves, channel, dom, to_mount)?;
            channel.hy_end_children();
        }

        Ok(())
    }
}

/// A flattened item at one DOM level. Components and fragments are
/// transparent — expanded into their constituent leaves.
#[derive(Clone, Copy)]
pub(super) enum Leaf<'a> {
    /// Static literal text from a `TemplateNode::Text`. `id` is `Some` only
    /// when this text is a *root* of some VNode.
    StaticText {
        text: &'a str,
        id: Option<ElementId>,
    },
    /// Runtime text from a `DynamicNode::Text`.
    DynamicText { value: &'a str, id: ElementId },
    /// A `TemplateNode::Element` plus the owning VNode (for resolving
    /// dynamic attribute slots).
    Element {
        vnode: &'a VNode,
        node: &'a TemplateNode,
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
    Element(Leaf<'a>),
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

/// Group leaves into emit steps. A text leaf greedy-extends across
/// consecutive text leaves AND across placeholders sandwiched between two
/// text leaves (the browser merges all text contributions into one DOM
/// node, with placeholders contributing zero HTML).
fn group_steps<'a, 'b>(leaves: &'b [Leaf<'a>]) -> Vec<EmitStep<'a, 'b>> {
    let mut steps = Vec::new();
    let mut i = 0;
    while i < leaves.len() {
        if leaves[i].is_text() {
            let mut end = i + 1;
            loop {
                while end < leaves.len() && leaves[end].is_text() {
                    end += 1;
                }
                break;
            }
            steps.push(EmitStep::TextRun(&leaves[i..end]));
            i = end;
        } else {
            match leaves[i] {
                Leaf::Element { .. } => steps.push(EmitStep::Element(leaves[i])),
                Leaf::StaticText { .. } | Leaf::DynamicText { .. } => {
                    unreachable!("text leaves handled above")
                }
            }
            i += 1;
        }
    }
    steps
}

/// True when the next *consuming* step after `idx` is a text run, meaning
/// our last non-empty text contribution needs `split_after` so the cursor
/// advances past it (instead of parking on it for the caller to advance 1).
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

/// Emit ops for one text run. All contributions share a single browser-
/// merged DOM text node (or zero, if every text contribution is empty).
///
/// For each non-empty text contribution we emit `hy_text_contrib`
/// (mapping + optional `splitText`). The last non-empty contribution
/// doesn't split unless `split_tail` is true (next consuming step is
/// another text run, so cursor must advance past).
///
/// Empty text contributions before the last non-empty get `hy_synth_text`
/// (virtual sentinel inserted before cursor). Empty contributions after
/// the last non-empty get `hy_synth_text_after` (virtual sentinel inserted
/// after cursor). Embedded placeholders use `hy_placeholder`.
///
/// Returns the number of DOM siblings this run consumed (0 if all-empty
/// or `split_tail`, else 1).
fn emit_text_run(run: &[Leaf<'_>], channel: &mut HydrationChannel, split_tail: bool) -> u32 {
    let last_nonempty = run.iter().rposition(leaf_text_is_non_empty);

    for (i, leaf) in run.iter().enumerate() {
        match *leaf {
            Leaf::StaticText { text, id } => {
                emit_text_leaf(channel, utf16_len(text), id, i, last_nonempty, split_tail);
            }
            Leaf::DynamicText { value, id } => {
                emit_text_leaf(
                    channel,
                    utf16_len(value),
                    Some(id),
                    i,
                    last_nonempty,
                    split_tail,
                );
            }
            Leaf::Element { .. } => unreachable!("non-text leaf in text run"),
        }
    }

    if last_nonempty.is_none() || split_tail {
        0
    } else {
        1
    }
}

fn emit_text_leaf(
    channel: &mut HydrationChannel,
    len: u32,
    id: Option<ElementId>,
    i: usize,
    last_nonempty: Option<usize>,
    split_tail: bool,
) {
    if len == 0 {
        let Some(id) = id else { return };
        // All-empty runs park every sentinel before the cursor (which is on
        // whatever follows the run). Otherwise position relative to the last
        // non-empty contribution: before goes _before_ the cursor, after goes
        // _after_ it.
        let before_cursor = match last_nonempty {
            None => true,
            Some(last) => i < last,
        };
        if before_cursor {
            channel.hy_synth_text(id.raw() as u32);
        } else {
            channel.hy_synth_text_after(id.raw() as u32);
        }
    } else {
        let id_arg = id.map(|i| i.raw() as u32).unwrap_or(0);
        let split_after = matches!(last_nonempty, Some(last) if i < last)
            || (split_tail && Some(i) == last_nonempty);
        channel.hy_text_contrib(len, id_arg, if split_after { 1 } else { 0 });
    }
}

fn leaf_text_is_non_empty(leaf: &Leaf<'_>) -> bool {
    match leaf {
        Leaf::StaticText { text, .. } => utf16_len(text) > 0,
        Leaf::DynamicText { value, .. } => utf16_len(value) > 0,
        _ => false,
    }
}

/// True if this template subtree contains any `Dynamic` slot or any
/// `Dynamic` attribute. Pure-static subtrees need no descent during
/// hydration — the parent's mapped element handles wrapper detection, and
/// the cursor steps over the subtree via `hy_advance(1)`.
fn template_node_has_dynamic_content(node: &TemplateNode) -> bool {
    match node {
        TemplateNode::Element {
            attrs, children, ..
        } => {
            attrs
                .iter()
                .any(|a| matches!(a, TemplateAttribute::Dynamic { .. }))
                || children.iter().any(template_node_has_dynamic_content)
        }
        TemplateNode::Text { .. } => false,
        TemplateNode::Dynamic { .. } => true,
    }
}

/// UTF-16 length, matching `Text.length` / `splitText` offsets in JS.
pub(super) fn utf16_len(s: &str) -> u32 {
    s.encode_utf16().count() as u32
}
