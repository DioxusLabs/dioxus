//! Virtual Node Support
//! --------------------
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers.
//!
//! These VNodes should be *very* cheap and *very* fast to construct - building a full tree should be insanely quick.
use crate::{
    events::VirtualEvent,
    innerlude::{Context, DomTree, Properties, RealDomNode, Scope, ScopeId, FC},
};
use std::{
    cell::Cell,
    fmt::{Arguments, Debug, Formatter},
    marker::PhantomData,
    rc::Rc,
};

pub struct VNode<'src> {
    pub kind: VNodeKind<'src>,
    pub(crate) dom_id: Cell<RealDomNode>,
    pub(crate) key: Option<&'src str>,
}
impl VNode<'_> {
    fn key(&self) -> Option<&str> {
        self.key
    }
}

/// Tools for the base unit of the virtual dom - the VNode
/// VNodes are intended to be quickly-allocated, lightweight enum values.
///
/// Components will be generating a lot of these very quickly, so we want to
/// limit the amount of heap allocations / overly large enum sizes.
pub enum VNodeKind<'src> {
    Text(VText<'src>),

    Element(&'src VElement<'src>),

    Fragment(VFragment<'src>),

    Component(&'src VComponent<'src>),

    Suspended { node: Rc<Cell<RealDomNode>> },
}

pub struct VText<'src> {
    pub text: &'src str,
    pub is_static: bool,
}

pub struct VFragment<'src> {
    pub children: &'src [VNode<'src>],
    pub is_static: bool,
    pub is_error: bool,
}

pub trait DioxusElement {
    const TAG_NAME: &'static str;
    const NAME_SPACE: Option<&'static str>;
    #[inline]
    fn tag_name(&self) -> &'static str {
        Self::TAG_NAME
    }
    #[inline]
    fn namespace(&self) -> Option<&'static str> {
        Self::NAME_SPACE
    }
}

pub struct VElement<'a> {
    // tag is always static
    pub tag_name: &'static str,
    pub namespace: Option<&'static str>,

    pub static_listeners: bool,
    pub listeners: &'a [Listener<'a>],

    pub static_attrs: bool,
    pub attributes: &'a [Attribute<'a>],

    pub static_children: bool,
    pub children: &'a [VNode<'a>],
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,

    pub value: &'a str,

    pub is_static: bool,

    pub is_volatile: bool,

    // Doesn't exist in the html spec, mostly used to denote "style" tags - could be for any type of group
    pub namespace: Option<&'static str>,
}

/// An event listener.
/// IE onclick, onkeydown, etc
pub struct Listener<'bump> {
    /// The type of event to listen for.
    pub(crate) event: &'static str,

    pub scope: ScopeId,

    pub mounted_node: &'bump mut Cell<RealDomNode>,

    pub(crate) callback: &'bump dyn FnMut(VirtualEvent),
}

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub struct VComponent<'src> {
    pub ass_scope: Cell<Option<ScopeId>>,

    pub(crate) caller: Rc<dyn Fn(&Scope) -> DomTree>,

    pub(crate) children: &'src [VNode<'src>],

    pub(crate) comparator: Option<&'src dyn Fn(&VComponent) -> bool>,

    pub is_static: bool,

    // a pointer into the bump arena (given by the 'src lifetime)
    pub(crate) raw_props: *const (),

    // a pointer to the raw fn typ
    pub(crate) user_fc: *const (),
}

/// This struct provides an ergonomic API to quickly build VNodes.
///
/// NodeFactory is used to build VNodes in the component's memory space.
/// This struct adds metadata to the final VNode about listeners, attributes, and children
#[derive(Copy, Clone)]
pub struct NodeFactory<'a> {
    pub scope_ref: &'a Scope,
    pub listener_id: &'a Cell<usize>,
}

impl<'a> NodeFactory<'a> {
    #[inline]
    pub fn bump(&self) -> &'a bumpalo::Bump {
        &self.scope_ref.cur_frame().bump
    }

    /// Used in a place or two to make it easier to build vnodes from dummy text
    pub fn static_text(text: &'static str) -> VNode {
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Text(VText {
                text,
                is_static: true,
            }),
        }
    }

    /// Parses a lazy text Arguments and returns a string and a flag indicating if the text is 'static
    ///
    /// Text that's static may be pointer compared, making it cheaper to diff
    pub fn raw_text(&self, args: Arguments) -> (&'a str, bool) {
        match args.as_str() {
            Some(static_str) => (static_str, true),
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(self.bump());
                s.write_fmt(args).unwrap();
                (s.into_bump_str(), false)
            }
        }
    }

    /// Create some text that's allocated along with the other vnodes
    ///
    pub fn text(&self, args: Arguments) -> VNode<'a> {
        let (text, is_static) = self.raw_text(args);
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Text(VText { text, is_static }),
        }
    }

    pub fn element(
        &self,
        el: impl DioxusElement,
        listeners: &'a mut [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [VNode<'a>],
        key: Option<&'a str>,
    ) -> VNode<'a> {
        self.raw_element(
            el.tag_name(),
            el.namespace(),
            listeners,
            attributes,
            children,
            key,
        )
    }

    pub fn raw_element(
        &self,
        tag: &'static str,
        namespace: Option<&'static str>,
        listeners: &'a mut [Listener],
        attributes: &'a [Attribute],
        children: &'a [VNode<'a>],
        key: Option<&'a str>,
    ) -> VNode<'a> {
        // We take the references directly from the bump arena
        // TODO: this code shouldn't necessarily be here of all places
        // It would make more sense to do this in diffing

        let mut queue = self.scope_ref.listeners.borrow_mut();
        for listener in listeners.iter_mut() {
            let mounted = listener.mounted_node as *mut _;
            let callback = listener.callback as *const _ as *mut _;
            queue.push((mounted, callback))
        }

        VNode {
            dom_id: RealDomNode::empty_cell(),
            key,
            kind: VNodeKind::Element(self.bump().alloc(VElement {
                tag_name: tag,
                namespace,
                listeners,
                attributes,
                children,

                // todo: wire up more constization
                static_listeners: false,
                static_attrs: false,
                static_children: false,
            })),
        }
    }

    pub fn suspended() -> VNode<'static> {
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Suspended {
                node: Rc::new(RealDomNode::empty_cell()),
            },
        }
    }

    pub fn attr(
        &self,
        name: &'static str,
        val: Arguments,
        namespace: Option<&'static str>,
        is_volatile: bool,
    ) -> Attribute<'a> {
        let (value, is_static) = self.raw_text(val);
        Attribute {
            name,
            value,
            is_static,
            namespace,
            is_volatile,
        }
    }

    pub fn attr_with_alloc_val(
        &self,
        name: &'static str,
        val: &'a str,
        namespace: Option<&'static str>,
        is_volatile: bool,
    ) -> Attribute<'a> {
        Attribute {
            name,
            value: val,
            is_static: false,
            namespace,
            is_volatile,
        }
    }

    pub fn component<P>(
        &self,
        component: FC<P>,
        props: P,
        key: Option<&'a str>,
        children: &'a [VNode<'a>],
    ) -> VNode<'a>
    where
        P: Properties + 'a,
    {
        // TODO
        // It's somewhat wrong to go about props like this

        // We don't want the fat part of the fat pointer
        // This function does static dispatch so we don't need any VTable stuff
        let props = self.bump().alloc(props);
        let raw_props = props as *const P as *const ();

        let user_fc = component as *const ();

        let comparator: Option<&dyn Fn(&VComponent) -> bool> = Some(self.bump().alloc_with(|| {
            move |other: &VComponent| {
                if user_fc == other.user_fc {
                    let real_other = unsafe { &*(other.raw_props as *const _ as *const P) };
                    let props_memoized = unsafe { props.memoize(&real_other) };
                    match (props_memoized, children.len() == 0) {
                        (true, true) => true,
                        _ => false,
                    }
                } else {
                    false
                }
            }
        }));

        let is_static = children.len() == 0 && P::IS_STATIC && key.is_none();

        VNode {
            key,
            dom_id: Cell::new(RealDomNode::empty()),
            kind: VNodeKind::Component(self.bump().alloc_with(|| VComponent {
                user_fc,
                comparator,
                raw_props,
                children,
                caller: NodeFactory::create_component_caller(component, raw_props),
                is_static,
                ass_scope: Cell::new(None),
            })),
        }
    }

    pub fn create_component_caller<'g, P: 'g>(
        component: FC<P>,
        raw_props: *const (),
    ) -> Rc<dyn for<'r> Fn(&'r Scope) -> DomTree<'r>> {
        type Captured<'a> = Rc<dyn for<'r> Fn(&'r Scope) -> DomTree<'r> + 'a>;
        let caller: Captured = Rc::new(move |scp: &Scope| -> DomTree {
            // cast back into the right lifetime
            let safe_props: &'_ P = unsafe { &*(raw_props as *const P) };
            let cx: Context<P> = Context {
                props: safe_props,
                scope: scp,
            };

            let res = component(cx);

            let g2 = unsafe { std::mem::transmute(res) };

            g2
        });
        unsafe { std::mem::transmute::<_, Captured<'static>>(caller) }
    }

    pub fn fragment_from_iter(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a>>,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump());

        for node in node_iter.into_iter() {
            nodes.push(node.into_vnode(self));
        }

        if cfg!(debug_assertions) {
            if nodes.len() > 1 {
                if nodes.last().unwrap().key().is_none() {
                    log::error!(
                        r#"
Warning: Each child in an array or iterator should have a unique "key" prop. 
Not providing a key will lead to poor performance with lists.
See docs.rs/dioxus for more information. 
---
To help you identify where this error is coming from, we've generated a backtrace.
                        "#,
                    );
                }
            }
        }
        VNode {
            dom_id: RealDomNode::empty_cell(),
            key: None,
            kind: VNodeKind::Fragment(VFragment {
                children: nodes.into_bump_slice(),
                is_static: false,
                is_error: false,
            }),
        }
    }
}

/// Trait implementations for use in the rsx! and html! macros.
///
/// ## Details
///
/// This section provides convenience methods and trait implementations for converting common structs into a format accepted
/// by the macros.
///
/// All dynamic content in the macros must flow in through `fragment_from_iter`. Everything else must be statically layed out.
/// We pipe basically everything through `fragment_from_iter`, so we expect a very specific type:
/// ```
/// impl IntoIterator<Item = impl IntoVNode<'a>>
/// ```
///
/// As such, all node creation must go through the factory, which is only availble in the component context.
/// These strict requirements make it possible to manage lifetimes and state.
pub trait IntoVNode<'a> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a>;
}

// For the case where a rendered VNode is passed into the rsx! macro through curly braces
impl<'a> IntoIterator for VNode<'a> {
    type Item = VNode<'a>;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

// For the case where a rendered VNode is passed into the rsx! macro through curly braces
impl<'a> IntoVNode<'a> for VNode<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        self
    }
}

// For the case where a rendered VNode is by reference passed into the rsx! macro through curly braces
// This behavior is designed for the cx.children method where child nodes are passed by reference.
//
// Designed to support indexing
impl<'a> IntoVNode<'a> for &VNode<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        let kind = match &self.kind {
            VNodeKind::Element(element) => VNodeKind::Element(element),
            VNodeKind::Text(old) => VNodeKind::Text(VText {
                text: old.text,
                is_static: old.is_static,
            }),
            VNodeKind::Fragment(fragment) => VNodeKind::Fragment(VFragment {
                children: fragment.children,
                is_static: fragment.is_static,
                is_error: false,
            }),
            VNodeKind::Component(component) => VNodeKind::Component(component),

            // todo: it doesn't make much sense to pass in suspended nodes
            // I think this is right but I'm not too sure.
            VNodeKind::Suspended { node } => VNodeKind::Suspended { node: node.clone() },
        };
        VNode {
            kind,
            dom_id: self.dom_id.clone(),
            key: self.key.clone(),
        }
    }
}

/// A concrete type provider for closures that build VNode structures.
///
/// This struct wraps lazy structs that build VNode trees Normally, we cannot perform a blanket implementation over
/// closures, but if we wrap the closure in a concrete type, we can maintain separate implementations of IntoVNode.
///
///
/// ```rust
/// LazyNodes::new(|f| f.element("div", [], [], [] None))
/// ```
pub struct LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    inner: G,
    _p: PhantomData<&'a ()>,
}

impl<'a, G> LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    pub fn new(f: G) -> Self {
        Self {
            inner: f,
            _p: PhantomData {},
        }
    }
}

// Our blanket impl
impl<'a, G> IntoVNode<'a> for LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        (self.inner)(cx)
    }
}

// Our blanket impl
impl<'a, G> IntoIterator for LazyNodes<'a, G>
where
    G: FnOnce(NodeFactory<'a>) -> VNode<'a>,
{
    type Item = Self;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

// Conveniently, we also support "null" (nothing) passed in
impl IntoVNode<'_> for () {
    fn into_vnode<'a>(self, cx: NodeFactory<'a>) -> VNode<'a> {
        cx.fragment_from_iter(None as Option<VNode>)
    }
}

// Conveniently, we also support "None"
impl IntoVNode<'_> for Option<()> {
    fn into_vnode<'a>(self, cx: NodeFactory<'a>) -> VNode<'a> {
        cx.fragment_from_iter(None as Option<VNode>)
    }
}
impl<'a> IntoVNode<'a> for Option<VNode<'a>> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        match self {
            Some(n) => n,
            None => cx.fragment_from_iter(None as Option<VNode>),
        }
    }
}

impl Debug for NodeFactory<'_> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self.kind {
            VNodeKind::Element(el) => write!(s, "element, {}", el.tag_name),
            VNodeKind::Text(t) => write!(s, "text, {}", t.text),
            VNodeKind::Fragment(_) => write!(s, "fragment"),
            VNodeKind::Suspended { .. } => write!(s, "suspended"),
            VNodeKind::Component(_) => write!(s, "component"),
        }
    }
}
