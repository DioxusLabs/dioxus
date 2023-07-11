use super::BoxedCellSlice;
use crate::{
    any_props::AnyProps, Attribute, Element, ElementId, ScopeId, ScopeState, Template, TemplateNode,
};
use std::{
    cell::{Cell, RefCell},
    fmt::Debug,
};
/// A reference to a template along with any context needed to hydrate it
///
/// The dynamic parts of the template are stored separately from the static parts. This allows faster diffing by skipping
/// static parts of the template.
#[derive(Debug, Clone)]
pub struct VNode<'a> {
    /// The key given to the root of this template.
    ///
    /// In fragments, this is the key of the first child. In other cases, it is the key of the root.
    pub key: Option<&'a str>,

    /// When rendered, this template will be linked to its parent manually
    pub parent: Option<ElementId>,

    /// The static nodes and static descriptor of the template
    pub template: Cell<Template<'static>>,

    /// The IDs for the roots of this template - to be used when moving the template around and removing it from
    /// the actual Dom
    pub root_ids: BoxedCellSlice,

    /// The dynamic parts of the template
    pub dynamic_nodes: &'a [DynamicNode<'a>],

    /// The dynamic parts of the template
    pub dynamic_attrs: &'a [Attribute<'a>],
}

impl<'a> VNode<'a> {
    /// Create a template with no nodes that will be skipped over during diffing
    pub fn empty() -> Element<'a> {
        panic!();
        Some(VNode {
            key: None,
            parent: None,
            root_ids: BoxedCellSlice::default(),
            dynamic_nodes: &[],
            dynamic_attrs: &[],
            template: Cell::new(Template {
                name: "dioxus-empty",
                roots: &[],
                node_paths: &[],
                attr_paths: &[],
            }),
        })
    }

    /// Create a template with a single placeholder node that will participate in diffing
    ///
    /// Used when components are rendered but escaped via `None`.
    pub fn placeholder(cx: &'a ScopeState) -> Element<'a> {
        panic!();
        Some(VNode {
            key: None,
            parent: None,
            root_ids: BoxedCellSlice::default(),
            dynamic_nodes: cx.bump().alloc([DynamicNode::default()]),
            dynamic_attrs: &[],
            template: Cell::new(Template {
                name: "dioxus-placeholder",
                roots: &[TemplateNode::Dynamic { id: 0 }],
                node_paths: &[],
                attr_paths: &[],
            }),
        })
    }

    /// Load a dynamic root at the given index
    ///
    /// Returns [`None`] if the root is actually a static node (Element/Text)
    pub fn dynamic_root(&self, idx: usize) -> Option<&'a DynamicNode<'a>> {
        match &self.template.get().roots[idx] {
            TemplateNode::Element { .. } | TemplateNode::Text { text: _ } => None,
            TemplateNode::Dynamic { id } | TemplateNode::DynamicText { id } => {
                Some(&self.dynamic_nodes[*id])
            }
        }
    }
}

/// A node created at runtime
///
/// This node's index in the DynamicNode list on VNode should match its repsective `Dynamic` index
#[derive(Debug)]
pub enum DynamicNode<'a> {
    /// A component node
    ///
    /// Most of the time, Dioxus will actually know which component this is as compile time, but the props and
    /// assigned scope are dynamic.
    ///
    /// The actual VComponent can be dynamic between two VNodes, though, allowing implementations to swap
    /// the render function at runtime
    Component(VComponent<'a>),

    /// A text node
    Text(VText<'a>),

    /// A placeholder
    ///
    /// Used by suspense when a node isn't ready and by fragments that don't render anything
    ///
    /// In code, this is just an ElementId whose initial value is set to 0 upon creation
    Placeholder(VPlaceholder),

    /// A list of VNodes.
    ///
    /// Note that this is not a list of dynamic nodes. These must be VNodes and created through conditional rendering
    /// or iterators.
    Fragment(&'a [VNode<'a>]),
}

impl Default for DynamicNode<'_> {
    fn default() -> Self {
        Self::Placeholder(Default::default())
    }
}

/// An instance of some text, mounted to the DOM
#[derive(Debug)]
pub struct VText<'a> {
    /// The actual text itself
    pub value: &'a str,

    /// The ID of this node in the real DOM
    pub id: Cell<Option<ElementId>>,
}

/// A placeholder node, used by suspense and fragments
#[derive(Debug, Default)]
pub struct VPlaceholder {
    /// The ID of this node in the real DOM
    pub id: Cell<Option<ElementId>>,
}

/// An instance of a child component
pub struct VComponent<'a> {
    /// The name of this component
    pub name: &'static str,

    /// Are the props valid for the 'static lifetime?
    ///
    /// Internally, this is used as a guarantee. Externally, this might be incorrect, so don't count on it.
    ///
    /// This flag is assumed by the [`crate::Properties`] trait which is unsafe to implement
    pub static_props: bool,

    /// The assigned Scope for this component
    pub scope: Cell<Option<ScopeId>>,

    /// The function pointer of the component, known at compile time
    ///
    /// It is possible that components get folded at comppile time, so these shouldn't be really used as a key
    pub render_fn: *const (),

    pub(crate) props: RefCell<Option<Box<dyn AnyProps<'a> + 'a>>>,
}

impl<'a> std::fmt::Debug for VComponent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VComponent")
            .field("name", &self.name)
            .field("static_props", &self.static_props)
            .field("scope", &self.scope)
            .finish()
    }
}
