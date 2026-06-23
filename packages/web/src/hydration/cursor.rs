//! A hydration cursor driven by the Rust VDOM walk.
//!
//! SSR emits no hydration markers. The Rust walker decides which template and
//! dynamic leaves should be matched; this cursor owns the actual DOM cursor -
//! sibling/child traversal, parser-inserted wrapper skipping, text splitting,
//! empty-slot materialization, node-id binding, and listener attachment.
//!
//! The DOM is read and mutated directly through `web-sys`. Only the two
//! operations that touch interpreter-internal state - binding an `ElementId`
//! to a node and registering an event listener - are delegated to the JS
//! `BaseInterpreter` via the generic `setNode` / `setNodeListener` primitives.

use dioxus_interpreter_js::unified_bindings::BaseInterpreter;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, Element, Node, Text};

use super::RehydrationError;
use super::RehydrationError::HydrationMismatch;

fn document() -> Document {
    web_sys::window().unwrap().document().unwrap()
}

/// A saved descent level, restored on ascent. `parent`/`resume` are the parent
/// and cursor to return to; `wrapper` marks a parser-inserted element the
/// template doesn't know about (popped transparently when ascending).
struct Ascent {
    parent: Node,
    resume: Node,
    wrapper: bool,
}

pub(super) struct HydrationCursor {
    base: JsValue,
    /// Current parent: matched children live here and synthesized nodes insert
    /// here. Always a concrete node - set on construction, updated on each
    /// descent and ascent.
    parent: Node,
    /// Current child position within `parent`. `None` once past the last child.
    cursor: Option<Node>,
    /// Saved descent levels (innermost last), restored by `end_children`.
    stack: Vec<Ascent>,
}

impl HydrationCursor {
    /// Park on the first server-rendered root. Returns `None` when SSR emitted no
    /// roots (after script filtering), so the caller can fall to hydrating inside
    /// the mount element via [`in_parent`](Self::in_parent).
    pub(super) fn over_roots(
        base: &BaseInterpreter,
        under: js_sys::Array,
        filter_scripts: bool,
    ) -> Result<Option<Self>, RehydrationError> {
        let Some(first) = under
            .iter()
            .map(|value| value.unchecked_into::<Node>())
            .find(|node| !(filter_scripts && is_script(node)))
        else {
            return Ok(None);
        };

        // An attached root always has a parent; a detached one is a real SSR
        // shape mismatch, not a case to paper over with a default parent.
        let parent = first.parent_node().ok_or(HydrationMismatch)?;

        Ok(Some(Self {
            base: base.unchecked_ref::<JsValue>().clone(),
            parent,
            cursor: Some(first),
            stack: Vec::new(),
        }))
    }

    /// Hydrate directly inside `parent` (used when SSR emitted no roots).
    pub(super) fn in_parent(base: &BaseInterpreter, parent: Node) -> Self {
        let cursor = parent.first_child();
        Self {
            base: base.unchecked_ref::<JsValue>().clone(),
            parent,
            cursor,
            stack: Vec::new(),
        }
    }

    fn base(&self) -> &BaseInterpreter {
        self.base.unchecked_ref()
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
            self.stack.push(Ascent {
                parent: self.parent.clone(),
                resume: node.clone(),
                wrapper: true,
            });
            self.parent = node;
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
        // Only reached after a successful `map_element`, so the cursor is set.
        let element = self.cursor.clone().expect("begin_children with no cursor");
        self.stack.push(Ascent {
            parent: self.parent.clone(),
            resume: element.clone(),
            wrapper: false,
        });
        self.cursor = element.first_child();
        self.parent = element;
    }

    /// Return to the parent level, popping any wrapper levels first.
    pub(super) fn end_children(&mut self) {
        while self.stack.last().is_some_and(|ascent| ascent.wrapper) {
            self.stack.pop();
        }
        let ascent = self
            .stack
            .pop()
            .expect("end_children without matching begin_children");
        self.parent = ascent.parent;
        self.cursor = Some(ascent.resume);
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

    /// Bind an addressable empty text slot at the current cursor position.
    ///
    /// SSR emits no bytes for an empty dynamic text, but the mounted VDOM may
    /// still need an ElementId for later updates. When core has already placed
    /// an empty text node for this slot (streaming suspense), claim it. Otherwise
    /// synthesize the anchor in the same position.
    pub(super) fn empty_text_slot(
        &mut self,
        id: u32,
        after_cursor: bool,
    ) -> Result<(), RehydrationError> {
        if let Some(node) = self.claimable_empty_text(after_cursor) {
            self.base().set_node(id, &node);
            if after_cursor {
                self.cursor = Some(node);
            } else {
                self.cursor = node.next_sibling();
            }
            return Ok(());
        }

        self.synth_empty_text(id, after_cursor)
    }

    /// Synthesize an empty text node around the current cursor.
    fn synth_empty_text(&mut self, id: u32, after_cursor: bool) -> Result<(), RehydrationError> {
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

    fn claimable_empty_text(&self, after_cursor: bool) -> Option<Node> {
        let node = if after_cursor {
            self.cursor.as_ref()?.next_sibling()?
        } else {
            self.cursor.clone()?
        };
        is_empty_text(&node).then_some(node)
    }

    /// The parent to insert synthesized nodes into.
    fn synth_parent(&self) -> &Node {
        &self.parent
    }
}

fn is_script(node: &Node) -> bool {
    node.node_type() == Node::ELEMENT_NODE
        && node.unchecked_ref::<Element>().local_name() == "script"
}

fn is_empty_text(node: &Node) -> bool {
    node.node_type() == Node::TEXT_NODE && node.unchecked_ref::<Text>().length() == 0
}
