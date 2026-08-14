//! A deliberately small retained DOM.
//!
//! There is no VDOM and no diffing: every dynamic part of the tree
//! (`dyn_text`, `dyn_attr`, `dyn_children`) is an [`crate::Effect`] that
//! patches the retained node in place, so update granularity comes from the
//! lens graph, not tree comparison. `dyn_children` rebuilds only its own
//! subtree, and because child effects are owned by the rebuilding effect,
//! everything the old subtree created is torn down automatically.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::rc::Rc;

use crate::effect::Effect;

type NodeRef = Rc<RefCell<DomNode>>;

enum DomNode {
    Element {
        tag: String,
        attrs: BTreeMap<String, String>,
        children: Vec<NodeRef>,
    },
    Text(String),
}

/// A handle to a retained DOM subtree. Cheap to clone (an `Rc`).
#[derive(Clone)]
pub struct Element {
    node: NodeRef,
}

/// Create an element node.
pub fn el(tag: &str) -> Element {
    Element {
        node: Rc::new(RefCell::new(DomNode::Element {
            tag: tag.to_string(),
            attrs: BTreeMap::new(),
            children: Vec::new(),
        })),
    }
}

/// Create a static text node.
pub fn text(content: impl Into<String>) -> Element {
    Element {
        node: Rc::new(RefCell::new(DomNode::Text(content.into()))),
    }
}

/// Create a text node bound to a reactive closure: it re-renders whenever a
/// value it read changes.
pub fn dyn_text(f: impl Fn() -> String + 'static) -> Element {
    let node: NodeRef = Rc::new(RefCell::new(DomNode::Text(String::new())));
    let target = node.clone();
    Effect::new(move || {
        *target.borrow_mut() = DomNode::Text(f());
    });
    Element { node }
}

impl Element {
    fn with_element<R>(
        &self,
        f: impl FnOnce(&mut BTreeMap<String, String>, &mut Vec<NodeRef>) -> R,
    ) -> R {
        match &mut *self.node.borrow_mut() {
            DomNode::Element {
                attrs, children, ..
            } => f(attrs, children),
            DomNode::Text(_) => panic!("text nodes have no attributes or children"),
        }
    }

    pub fn attr(self, name: &str, value: impl Into<String>) -> Self {
        self.with_element(|attrs, _| {
            attrs.insert(name.to_string(), value.into());
        });
        self
    }

    /// Bind an attribute to a reactive closure.
    pub fn dyn_attr(self, name: &str, f: impl Fn() -> String + 'static) -> Self {
        let name = name.to_string();
        let target = self.node.clone();
        Effect::new(move || {
            let value = f();
            if let DomNode::Element { attrs, .. } = &mut *target.borrow_mut() {
                attrs.insert(name.clone(), value);
            }
        });
        self
    }

    pub fn child(self, child: Element) -> Self {
        self.with_element(|_, children| children.push(child.node.clone()));
        self
    }

    pub fn children(self, iter: impl IntoIterator<Item = Element>) -> Self {
        self.with_element(|_, children| {
            children.extend(iter.into_iter().map(|c| c.node));
        });
        self
    }

    /// Bind this element's child list to a reactive closure. The closure owns
    /// everything it builds: nested effects created for the previous children
    /// are dropped before each rebuild.
    pub fn dyn_children(self, f: impl Fn() -> Vec<Element> + 'static) -> Self {
        let target = self.node.clone();
        Effect::new(move || {
            let new_children: Vec<NodeRef> = f().into_iter().map(|c| c.node).collect();
            if let DomNode::Element { children, .. } = &mut *target.borrow_mut() {
                *children = new_children;
            }
        });
        self
    }

    /// Render the subtree to an HTML-ish string (for tests and terminals).
    pub fn render_to_string(&self) -> String {
        let mut out = String::new();
        render_node(&self.node, &mut out);
        out
    }
}

fn render_node(node: &NodeRef, out: &mut String) {
    match &*node.borrow() {
        DomNode::Text(content) => out.push_str(content),
        DomNode::Element {
            tag,
            attrs,
            children,
        } => {
            let _ = write!(out, "<{tag}");
            for (name, value) in attrs {
                let _ = write!(out, " {name}=\"{value}\"");
            }
            out.push('>');
            for child in children {
                render_node(child, out);
            }
            let _ = write!(out, "</{tag}>");
        }
    }
}
