//! A hydration cursor driven by the Rust VDOM walk.
//!
//! SSR emits no hydration markers. The Rust walker decides which template and
//! dynamic leaves should be matched; this cursor owns the actual DOM cursor —
//! sibling/child traversal, parser-inserted wrapper skipping, text splitting,
//! synthesized empty text nodes, node-id binding, and listener attachment.
//!
//! The DOM is read and mutated directly through `web-sys`. Only the two
//! operations that touch interpreter-internal state — binding an `ElementId`
//! to a node and registering an event listener — are delegated to the JS
//! `BaseInterpreter` via the generic `setNode` / `setNodeListener` primitives.

use dioxus_interpreter_js::unified_bindings::BaseInterpreter;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Element, Node, Text};

use super::RehydrationError;
use super::RehydrationError::HydrationMismatch;

fn document() -> Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A parent on the descent stack. `wrapper` marks a parser-inserted element the
/// template doesn't know about (skipped over when ascending).
struct HydrationFrame {
    node: Option<Node>,
    wrapper: bool,
}

pub(super) struct HydrationCursor {
    base: JsValue,
    /// The DOM node the cursor currently points at.
    cursor: Option<Node>,
    /// Parent stack accumulated while descending into children.
    frames: Vec<HydrationFrame>,
    /// Top-level nodes that roots map into (scripts optionally filtered out).
    under: Vec<Node>,
    /// The hydration container; the insertion fallback for synthesized nodes.
    root: Node,
    /// The current root's parent, used as a synth-insertion fallback.
    current_root_parent: Node,
}

impl HydrationCursor {
    pub(super) fn new(
        base: &BaseInterpreter,
        root: Node,
        under: js_sys::Array,
        filter_scripts: bool,
    ) -> Self {
        let mut nodes = Vec::with_capacity(under.length() as usize);
        for value in under.iter() {
            let node: Node = value.unchecked_into();
            if filter_scripts && is_script(&node) {
                continue;
            }
            nodes.push(node);
        }

        Self {
            base: base.unchecked_ref::<JsValue>().clone(),
            cursor: None,
            frames: Vec::new(),
            under: nodes,
            current_root_parent: root.clone(),
            root,
        }
    }

    fn base(&self) -> &BaseInterpreter {
        self.base.unchecked_ref()
    }

    pub(super) fn root_count(&self) -> u32 {
        self.under.len() as u32
    }

    /// Park the cursor on a root node.
    pub(super) fn enter_root(&mut self, idx: usize) {
        let node = self.under.get(idx).cloned();
        self.current_root_parent = node
            .as_ref()
            .and_then(|n| n.parent_node())
            .unwrap_or_else(|| self.root.clone());
        self.cursor = node;
        self.frames.clear();
    }

    /// Match and bind an element at the current cursor, stepping past
    /// parser-inserted wrappers (unrecognized, attribute-less elements).
    pub(super) fn map_element(&mut self, tag: &str, id: u32) -> Result<(), RehydrationError> {
        while let Some(node) = self.cursor.clone() {
            if node.node_type() != Node::ELEMENT_NODE {
                break;
            }
            let element = node.unchecked_ref::<Element>();
            if element.local_name() == tag || element.has_attributes() {
                break;
            }
            let first = node.first_child();
            self.frames.push(HydrationFrame {
                node: Some(node),
                wrapper: true,
            });
            self.cursor = first;
        }

        let node = self.cursor.clone().ok_or(HydrationMismatch)?;
        if node.node_type() != Node::ELEMENT_NODE {
            return Err(HydrationMismatch);
        }
        if node.unchecked_ref::<Element>().local_name() != tag {
            return Err(HydrationMismatch);
        }

        // id == 0 means the element needs no node binding but still occupies a
        // positional slot (the tag check above still verifies its shape).
        if id != 0 {
            self.base().set_node(id, &node);
        }
        Ok(())
    }

    /// Attach a listener to the element most recently mapped under `id`.
    pub(super) fn attach_listener(&self, id: u32, name: &str, bubbles: bool) {
        self.base().set_node_listener(id, name, bubbles);
    }

    /// Descend into the current cursor's children.
    pub(super) fn begin_children(&mut self) {
        let frame = self.cursor.clone();
        self.cursor = frame.as_ref().and_then(|n| n.first_child());
        self.frames.push(HydrationFrame {
            node: frame,
            wrapper: false,
        });
    }

    /// Return to the parent cursor, popping any wrapper frames first.
    pub(super) fn end_children(&mut self) {
        while self.frames.last().is_some_and(|f| f.wrapper) {
            self.frames.pop();
        }
        self.cursor = self.frames.pop().and_then(|f| f.node);
    }

    /// Advance through `n` sibling DOM nodes.
    pub(super) fn advance(&mut self, n: u32) {
        for _ in 0..n {
            let next = match self.cursor.as_ref() {
                Some(node) => node.next_sibling(),
                None => break,
            };
            self.cursor = next;
        }
    }

    /// Bind a text contribution and optionally split the browser-merged text node.
    pub(super) fn text_contrib(
        &mut self,
        len: u32,
        id: u32,
        split_after: bool,
    ) -> Result<(), RehydrationError> {
        let node = self.cursor.clone().ok_or(HydrationMismatch)?;
        if node.node_type() != Node::TEXT_NODE {
            return Err(HydrationMismatch);
        }
        if id != 0 {
            self.base().set_node(id, &node);
        }
        if split_after {
            let rest = node
                .unchecked_ref::<Text>()
                .split_text(len)
                .map_err(|_| HydrationMismatch)?;
            self.cursor = Some(rest.unchecked_into());
        }
        Ok(())
    }

    /// Synthesize an empty text node around the current cursor.
    pub(super) fn synth(&mut self, id: u32, after_cursor: bool) -> Result<(), RehydrationError> {
        let before = if after_cursor {
            self.cursor.as_ref().and_then(|c| c.next_sibling())
        } else {
            self.cursor.clone()
        };
        let node: Node = document().create_text_node("").unchecked_into();
        self.synth_parent()
            .insert_before(&node, before.as_ref())
            .map_err(|_| HydrationMismatch)?;
        self.base().set_node(id, &node);
        if after_cursor {
            self.cursor = Some(node);
        }
        Ok(())
    }

    /// The parent to insert synthesized nodes into: the cursor's parent, else
    /// the innermost frame, else the current root's parent.
    fn synth_parent(&self) -> Node {
        self.cursor
            .as_ref()
            .and_then(|c| c.parent_node())
            .or_else(|| self.frames.last().and_then(|f| f.node.clone()))
            .unwrap_or_else(|| self.current_root_parent.clone())
    }
}

fn is_script(node: &Node) -> bool {
    node.node_type() == Node::ELEMENT_NODE
        && node.unchecked_ref::<Element>().local_name() == "script"
}
