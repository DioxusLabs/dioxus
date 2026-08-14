//! A minimal retained DOM. There is no virtual DOM and no diffing: dynamic
//! text, attributes, and children are bound with effects that surgically
//! mutate the retained tree when their lens/memo dependencies change.

use crate::ui::Ctx;

/// Index of a retained node in the [`Dom`] arena.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(usize);

pub(crate) enum RetainedNode {
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<NodeId>,
    },
    Text(String),
    /// An anonymous container for a dynamic child list.
    Fragment {
        children: Vec<NodeId>,
    },
}

/// The retained node arena. Owned by [`crate::Ui`]; mutated by binding
/// effects through [`Ctx`].
pub struct Dom<S: 'static> {
    pub(crate) nodes: Vec<RetainedNode>,
    _marker: std::marker::PhantomData<fn() -> S>,
}

impl<S: 'static> Default for Dom<S> {
    fn default() -> Self {
        Dom {
            nodes: Vec::new(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S: 'static> Dom<S> {
    fn push(&mut self, node: RetainedNode) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub(crate) fn set_text(&mut self, id: NodeId, content: String) {
        if let RetainedNode::Text(t) = &mut self.nodes[id.0] {
            *t = content;
        }
    }

    pub(crate) fn set_attr(&mut self, id: NodeId, name: &str, value: String) {
        if let RetainedNode::Element { attrs, .. } = &mut self.nodes[id.0] {
            if let Some(slot) = attrs.iter_mut().find(|(n, _)| n == name) {
                slot.1 = value;
            } else {
                attrs.push((name.to_string(), value));
            }
        }
    }

    pub(crate) fn set_fragment_children(&mut self, id: NodeId, children: Vec<NodeId>) {
        if let RetainedNode::Fragment { children: c } = &mut self.nodes[id.0] {
            *c = children;
        }
    }

    /// Render a retained node (and its subtree) to an HTML string.
    pub(crate) fn render_to_string(&self, id: NodeId) -> String {
        let mut out = String::new();
        self.write_node(id, &mut out);
        out
    }

    fn write_node(&self, id: NodeId, out: &mut String) {
        match &self.nodes[id.0] {
            RetainedNode::Text(t) => out.push_str(t),
            RetainedNode::Fragment { children } => {
                for child in children {
                    self.write_node(*child, out);
                }
            }
            RetainedNode::Element {
                tag,
                attrs,
                children,
            } => {
                out.push('<');
                out.push_str(tag);
                for (name, value) in attrs {
                    out.push(' ');
                    out.push_str(name);
                    out.push_str("=\"");
                    out.push_str(value);
                    out.push('"');
                }
                out.push('>');
                for child in children {
                    self.write_node(*child, out);
                }
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
        }
    }
}

type TextFn<S> = Box<dyn FnMut(&mut Ctx<'_, S>) -> String>;
type ChildrenFn<S> = Box<dyn FnMut(&mut Ctx<'_, S>) -> Vec<Node<S>>>;

enum Attr<S: 'static> {
    Static(String, String),
    Dyn(String, TextFn<S>),
}

/// A DOM tree description, consumed by [`mount`].
pub enum Node<S: 'static> {
    /// An element with attributes and children.
    Element(El<S>),
    /// A static text node.
    Text(String),
    /// A reactive text node bound by an effect.
    DynText(TextFn<S>),
    /// A reactive child list rebuilt by an effect.
    DynChildren(ChildrenFn<S>),
}

/// An element description with static and dynamic attributes and children.
pub struct El<S: 'static> {
    tag: String,
    attrs: Vec<Attr<S>>,
    children: Vec<Node<S>>,
}

/// Describe an element.
pub fn el<S: 'static>(tag: &str) -> El<S> {
    El {
        tag: tag.to_string(),
        attrs: Vec::new(),
        children: Vec::new(),
    }
}

/// Describe a static text node.
pub fn text<S: 'static>(content: impl Into<String>) -> Node<S> {
    Node::Text(content.into())
}

/// Describe a reactive text node. `f` runs inside an effect: whatever it
/// reads through its [`Ctx`] becomes a dependency, and the retained text is
/// updated in place when a dependency changes.
pub fn dyn_text<S: 'static>(f: impl FnMut(&mut Ctx<'_, S>) -> String + 'static) -> Node<S> {
    Node::DynText(Box::new(f))
}

impl<S: 'static> El<S> {
    /// Add a static attribute.
    pub fn attr(mut self, name: &str, value: impl Into<String>) -> Self {
        self.attrs
            .push(Attr::Static(name.to_string(), value.into()));
        self
    }

    /// Add a reactive attribute, updated in place by an effect.
    pub fn dyn_attr(
        mut self,
        name: &str,
        f: impl FnMut(&mut Ctx<'_, S>) -> String + 'static,
    ) -> Self {
        self.attrs.push(Attr::Dyn(name.to_string(), Box::new(f)));
        self
    }

    /// Append a child node.
    pub fn child(mut self, node: impl Into<Node<S>>) -> Self {
        self.children.push(node.into());
        self
    }

    /// Append several child nodes.
    pub fn children(mut self, nodes: impl IntoIterator<Item = Node<S>>) -> Self {
        self.children.extend(nodes);
        self
    }

    /// Append a reactive child list. `f` runs inside an effect: when a
    /// dependency changes the previous children (and any effects they
    /// registered) are torn down and the list is rebuilt.
    pub fn dyn_children(
        mut self,
        f: impl FnMut(&mut Ctx<'_, S>) -> Vec<Node<S>> + 'static,
    ) -> Self {
        self.children.push(Node::DynChildren(Box::new(f)));
        self
    }
}

impl<S: 'static> From<El<S>> for Node<S> {
    fn from(el: El<S>) -> Self {
        Node::Element(el)
    }
}

/// Mount a node description into the retained arena, registering binding
/// effects for all dynamic parts. Effects created here are owned by the
/// current observer, so a dynamic subtree is torn down with its creator.
pub fn mount<S: 'static>(ctx: &mut Ctx<'_, S>, node: Node<S>) -> NodeId {
    match node {
        Node::Text(t) => ctx.dom.push(RetainedNode::Text(t)),
        Node::DynText(mut f) => {
            let id = ctx.dom.push(RetainedNode::Text(String::new()));
            ctx.effect(move |ctx| {
                let content = f(ctx);
                ctx.dom.set_text(id, content);
            });
            id
        }
        Node::DynChildren(mut f) => {
            let id = ctx.dom.push(RetainedNode::Fragment {
                children: Vec::new(),
            });
            ctx.effect(move |ctx| {
                let nodes = f(ctx);
                let children: Vec<NodeId> =
                    nodes.into_iter().map(|node| mount(ctx, node)).collect();
                ctx.dom.set_fragment_children(id, children);
            });
            id
        }
        Node::Element(element) => {
            let id = ctx.dom.push(RetainedNode::Element {
                tag: element.tag,
                attrs: Vec::new(),
                children: Vec::new(),
            });
            for attr in element.attrs {
                match attr {
                    Attr::Static(name, value) => ctx.dom.set_attr(id, &name, value),
                    Attr::Dyn(name, mut f) => {
                        ctx.effect(move |ctx| {
                            let value = f(ctx);
                            ctx.dom.set_attr(id, &name, value);
                        });
                    }
                }
            }
            let children: Vec<NodeId> = element
                .children
                .into_iter()
                .map(|child| mount(ctx, child))
                .collect();
            if let RetainedNode::Element { children: c, .. } = &mut ctx.dom.nodes[id.0] {
                *c = children;
            }
            id
        }
    }
}
