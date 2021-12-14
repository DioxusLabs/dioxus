//! Virtual Node Support
//!
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers. These VNodes should be *very*
//! cheap and *very* fast to construct - building a full tree should be quick.

use crate::{
    innerlude::{Element, Properties, Scope, ScopeId, ScopeState},
    lazynodes::LazyNodes,
};
use bumpalo::{boxed::Box as BumpBox, Bump};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug, Formatter},
    sync::Arc,
};

/// A composable "VirtualNode" to declare a User Interface in the Dioxus VirtualDOM.
///
/// VNodes are designed to be lightweight and used with with a bump allocator. To create a VNode, you can use either of:
///
/// - the [`rsx`] macro
/// - the [`NodeFactory`] API
pub enum VNode<'src> {
    /// Text VNodes simply bump-allocated (or static) string slices
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut vdom = VirtualDom::new();
    /// let node = vdom.render_vnode(rsx!( "hello" ));
    ///
    /// if let VNode::Text(vtext) = node {
    ///     assert_eq!(vtext.text, "hello");
    ///     assert_eq!(vtext.dom_id.get(), None);
    ///     assert_eq!(vtext.is_static, true);
    /// }
    /// ```
    Text(&'src VText<'src>),

    /// Element VNodes are VNodes that may contain attributes, listeners, a key, a tag, and children.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut vdom = VirtualDom::new();
    ///
    /// let node = vdom.render_vnode(rsx!{
    ///     div {
    ///         key: "a",
    ///         onclick: |e| log::info!("clicked"),
    ///         hidden: "true",
    ///         style: { background_color: "red" }
    ///         "hello"
    ///     }
    /// });
    ///
    /// if let VNode::Element(velement) = node {
    ///     assert_eq!(velement.tag_name, "div");
    ///     assert_eq!(velement.namespace, None);
    ///     assert_eq!(velement.key, Some("a));
    /// }
    /// ```
    Element(&'src VElement<'src>),

    /// Fragment nodes may contain many VNodes without a single root.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// rsx!{
    ///     a {}
    ///     link {}
    ///     style {}
    ///     "asd"
    ///     Example {}
    /// }
    /// ```
    Fragment(VFragment<'src>),

    /// Component nodes represent a mounted component with props, children, and a key.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// fn Example(cx: Context, props: &()) -> Element {
    ///     todo!()
    /// }
    ///
    /// let mut vdom = VirtualDom::new();
    ///
    /// let node = vdom.render_vnode(rsx!( Example {} ));
    ///
    /// if let VNode::Component(vcomp) = node {
    ///     assert_eq!(vcomp.user_fc, Example as *const ());
    /// }
    /// ```
    Component(&'src VComponent<'src>),

    /// Placeholders are a type of placeholder VNode used when fragments don't contain any children.
    ///
    /// Placeholders cannot be directly constructed via public APIs.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// let mut vdom = VirtualDom::new();
    ///
    /// let node = vdom.render_vnode(rsx!( Fragment {} ));
    ///
    /// if let VNode::Fragment(frag) = node {
    ///     let root = &frag.children[0];
    ///     assert_eq!(root, VNode::Anchor);
    /// }
    /// ```
    Placeholder(&'src VPlaceholder),

    /// A VNode that is actually a pointer to some nodes rather than the nodes directly. Useful when rendering portals
    /// or eliding lifetimes on VNodes through runtime checks.
    ///
    /// Linked VNodes can only be made through the [`Context::render`] method
    ///
    /// Typically, linked nodes are found *not* in a VNode. When NodeLinks are in a VNode, the NodeLink was passed into
    /// an `rsx!` call.
    ///
    /// # Example
    /// ```rust, ignore
    /// let mut vdom = VirtualDom::new();
    ///
    /// let node: NodeLink = vdom.render_vnode(rsx!( "hello" ));
    /// ```
    Portal(VPortal),
}

impl<'src> VNode<'src> {
    /// Get the VNode's "key" used in the keyed diffing algorithm.
    pub fn key(&self) -> Option<&'src str> {
        match &self {
            VNode::Element(el) => el.key,
            VNode::Component(c) => c.key,
            VNode::Fragment(f) => f.key,
            VNode::Text(_t) => None,
            VNode::Placeholder(_f) => None,
            VNode::Portal(_c) => None,
        }
    }

    /// Get the ElementID of the mounted VNode.
    ///
    /// Panics if the mounted ID is None or if the VNode is not represented by a single Element.
    pub fn mounted_id(&self) -> ElementId {
        self.try_mounted_id().unwrap()
    }

    /// Try to get the ElementID of the mounted VNode.
    ///
    /// Returns None if the VNode is not mounted, or if the VNode cannot be presented by a mounted ID (Fragment/Component)
    pub fn try_mounted_id(&self) -> Option<ElementId> {
        match &self {
            VNode::Text(el) => el.dom_id.get(),
            VNode::Element(el) => el.dom_id.get(),
            VNode::Placeholder(el) => el.dom_id.get(),
            VNode::Portal(_) => None,
            VNode::Fragment(_) => None,
            VNode::Component(_) => None,
        }
    }

    pub(crate) fn children(&self) -> &[VNode<'src>] {
        match &self {
            VNode::Fragment(f) => f.children,
            _ => &[],
        }
    }

    // Create an "owned" version of the vnode.
    pub fn decouple(&self) -> VNode<'src> {
        match self {
            VNode::Text(t) => VNode::Text(*t),
            VNode::Element(e) => VNode::Element(*e),
            VNode::Component(c) => VNode::Component(*c),
            VNode::Placeholder(a) => VNode::Placeholder(*a),
            VNode::Fragment(f) => VNode::Fragment(VFragment {
                children: f.children,
                key: f.key,
            }),
            VNode::Portal(c) => VNode::Portal(VPortal {
                scope_id: c.scope_id.clone(),
                link_idx: c.link_idx.clone(),
                node: c.node,
            }),
        }
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self {
            VNode::Element(el) => s
                .debug_struct("VNode::VElement")
                .field("name", &el.tag_name)
                .field("key", &el.key)
                .finish(),
            VNode::Text(t) => write!(s, "VNode::VText {{ text: {} }}", t.text),
            VNode::Placeholder(_) => write!(s, "VNode::VAnchor"),
            VNode::Fragment(frag) => {
                write!(s, "VNode::VFragment {{ children: {:?} }}", frag.children)
            }
            VNode::Component(comp) => write!(s, "VNode::VComponent {{ fc: {:?}}}", comp.user_fc),
            VNode::Portal(c) => write!(s, "VNode::VCached {{ scope_id: {:?} }}", c.scope_id.get()),
        }
    }
}

/// An Element's unique identifier.
///
/// `ElementId` is a `usize` that is unique across the entire VirtualDOM - but not unique across time. If a component is
/// unmounted, then the `ElementId` will be reused for a new component.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ElementId(pub usize);
impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ElementId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

fn empty_cell() -> Cell<Option<ElementId>> {
    Cell::new(None)
}

/// A placeholder node only generated when Fragments don't have any children.
pub struct VPlaceholder {
    pub dom_id: Cell<Option<ElementId>>,
}

/// A bump-allocated string slice and metadata.
pub struct VText<'src> {
    pub text: &'src str,
    pub dom_id: Cell<Option<ElementId>>,
    pub is_static: bool,
}

/// A list of VNodes with no single root.
pub struct VFragment<'src> {
    pub key: Option<&'src str>,
    pub children: &'src [VNode<'src>],
}

/// An element like a "div" with children, listeners, and attributes.
pub struct VElement<'a> {
    pub tag_name: &'static str,
    pub namespace: Option<&'static str>,
    pub key: Option<&'a str>,
    pub dom_id: Cell<Option<ElementId>>,
    pub parent_id: Cell<Option<ElementId>>,
    pub listeners: &'a [Listener<'a>],
    pub attributes: &'a [Attribute<'a>],
    pub children: &'a [VNode<'a>],
}

impl Debug for VElement<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VElement")
            .field("tag_name", &self.tag_name)
            .field("namespace", &self.namespace)
            .field("key", &self.key)
            .field("dom_id", &self.dom_id)
            .field("parent_id", &self.parent_id)
            .field("listeners", &self.listeners.len())
            .field("attributes", &self.attributes)
            .field("children", &self.children)
            .finish()
    }
}

/// A trait for any generic Dioxus Element.
///
/// This trait provides the ability to use custom elements in the `rsx!` macro.
///
/// ```rust, ignore
/// struct my_element;
///
/// impl DioxusElement for my_element {
///     const TAG_NAME: "my_element";
///     const NAME_SPACE: None;
/// }
///
/// let _ = rsx!{
///     my_element {}
/// };
/// ```
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

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'static str,

    pub value: &'a str,

    pub is_static: bool,

    pub is_volatile: bool,

    // Doesn't exist in the html spec.
    // Used in Dioxus to denote "style" tags.
    pub namespace: Option<&'static str>,
}

/// An event listener.
/// IE onclick, onkeydown, etc
pub struct Listener<'bump> {
    /// The ID of the node that this listener is mounted to
    /// Used to generate the event listener's ID on the DOM
    pub mounted_node: Cell<Option<ElementId>>,

    /// The type of event to listen for.
    ///
    /// IE "click" - whatever the renderer needs to attach the listener by name.
    pub event: &'static str,

    /// The actual callback that the user specified
    pub(crate) callback: EventHandler<'bump>,
}

/// The callback based into element event listeners.
pub struct EventHandler<'bump> {
    pub callback: &'bump RefCell<Option<ListenerCallback<'bump>>>,
}

impl EventHandler<'_> {
    pub fn call(&self, event: Arc<dyn Any + Send + Sync>) {
        if let Some(callback) = self.callback.borrow_mut().as_mut() {
            callback(event);
        }
    }
    pub fn release(&self) {
        self.callback.replace(None);
    }
}

type ListenerCallback<'bump> = BumpBox<'bump, dyn FnMut(Arc<dyn Any + Send + Sync>) + 'bump>;

impl Copy for EventHandler<'_> {}
impl Clone for EventHandler<'_> {
    fn clone(&self) -> Self {
        Self {
            callback: self.callback,
        }
    }
}

/// A cached node is a "pointer" to a "rendered" node in a particular scope
///
/// It does not provide direct access to the node, so it doesn't carry any lifetime information with it
///
/// It is used during the diffing/rendering process as a runtime key into an existing set of nodes. The "render" key
/// is essentially a unique key to guarantee safe usage of the Node.
///
/// Linked VNodes can only be made through the [`Context::render`] method
///
/// Typically, NodeLinks are found *not* in a VNode. When NodeLinks are in a VNode, the NodeLink was passed into
/// an `rsx!` call.
///
/// todo: remove the raw pointer and use runtime checks instead
#[derive(Debug)]
pub struct VPortal {
    pub(crate) link_idx: Cell<usize>,
    pub(crate) scope_id: Cell<Option<ScopeId>>,
    pub(crate) node: *const VNode<'static>,
}

impl PartialEq for VPortal {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub struct VComponent<'src> {
    pub key: Option<&'src str>,

    pub associated_scope: Cell<Option<ScopeId>>,

    // Function pointer to the FC that was used to generate this component
    pub user_fc: *const (),

    pub(crate) can_memoize: bool,

    pub(crate) _hard_allocation: Cell<Option<*const ()>>,

    // Raw pointer into the bump arena for the props of the component
    pub(crate) bump_props: *const (),

    // during the "teardown" process we'll take the caller out so it can be dropped properly
    pub(crate) caller: &'src dyn Fn(&'src ScopeState) -> Element,

    pub(crate) comparator: Option<&'src dyn Fn(&VComponent) -> bool>,

    // todo: re-add this
    pub(crate) drop_props: RefCell<Option<BumpBox<'src, dyn FnMut()>>>,
}

/// This struct provides an ergonomic API to quickly build VNodes.
///
/// NodeFactory is used to build VNodes in the component's memory space.
/// This struct adds metadata to the final VNode about listeners, attributes, and children
#[derive(Copy, Clone)]
pub struct NodeFactory<'a> {
    pub(crate) bump: &'a Bump,
}

impl<'a> NodeFactory<'a> {
    pub fn new(bump: &'a Bump) -> NodeFactory<'a> {
        NodeFactory { bump }
    }

    #[inline]
    pub fn bump(&self) -> &'a bumpalo::Bump {
        self.bump
    }

    /// Directly pass in text blocks without the need to use the format_args macro.
    pub fn static_text(&self, text: &'static str) -> VNode<'a> {
        VNode::Text(self.bump.alloc(VText {
            dom_id: empty_cell(),
            text,
            is_static: true,
        }))
    }

    /// Parses a lazy text Arguments and returns a string and a flag indicating if the text is 'static
    ///
    /// Text that's static may be pointer compared, making it cheaper to diff
    pub fn raw_text(&self, args: Arguments) -> (&'a str, bool) {
        match args.as_str() {
            Some(static_str) => (static_str, true),
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut str_buf = bumpalo::collections::String::new_in(self.bump);
                str_buf.write_fmt(args).unwrap();
                (str_buf.into_bump_str(), false)
            }
        }
    }

    /// Create some text that's allocated along with the other vnodes
    ///
    pub fn text(&self, args: Arguments) -> VNode<'a> {
        let (text, is_static) = self.raw_text(args);

        VNode::Text(self.bump.alloc(VText {
            text,
            is_static,
            dom_id: empty_cell(),
        }))
    }

    pub fn element<L, A, V>(
        &self,
        el: impl DioxusElement,
        listeners: L,
        attributes: A,
        children: V,
        key: Option<Arguments>,
    ) -> VNode<'a>
    where
        L: 'a + AsRef<[Listener<'a>]>,
        A: 'a + AsRef<[Attribute<'a>]>,
        V: 'a + AsRef<[VNode<'a>]>,
    {
        self.raw_element(
            el.tag_name(),
            el.namespace(),
            listeners,
            attributes,
            children,
            key,
        )
    }

    pub fn raw_element<L, A, V>(
        &self,
        tag_name: &'static str,
        namespace: Option<&'static str>,
        listeners: L,
        attributes: A,
        children: V,
        key: Option<Arguments>,
    ) -> VNode<'a>
    where
        L: 'a + AsRef<[Listener<'a>]>,
        A: 'a + AsRef<[Attribute<'a>]>,
        V: 'a + AsRef<[VNode<'a>]>,
    {
        let listeners: &'a L = self.bump.alloc(listeners);
        let listeners = listeners.as_ref();

        let attributes: &'a A = self.bump.alloc(attributes);
        let attributes = attributes.as_ref();

        let children: &'a V = self.bump.alloc(children);
        let children = children.as_ref();

        let key = key.map(|f| self.raw_text(f).0);

        VNode::Element(self.bump.alloc(VElement {
            tag_name,
            key,
            namespace,
            listeners,
            attributes,
            children,
            dom_id: empty_cell(),
            parent_id: empty_cell(),
        }))
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

    pub fn component<P>(
        &self,
        component: fn(Scope<'a, P>) -> Element,
        props: P,
        key: Option<Arguments>,
    ) -> VNode<'a>
    where
        P: Properties + 'a,
    {
        let bump = self.bump;
        let props = bump.alloc(props);
        let bump_props = props as *mut P as *mut ();
        let user_fc = component as *const ();

        let comparator: &mut dyn Fn(&VComponent) -> bool = bump.alloc_with(|| {
            move |other: &VComponent| {
                if user_fc == other.user_fc {
                    // Safety
                    // - We guarantee that FC<P> is the same by function pointer
                    // - Because FC<P> is the same, then P must be the same (even with generics)
                    // - Non-static P are autoderived to memoize as false
                    // - This comparator is only called on a corresponding set of bumpframes
                    //
                    // It's only okay to memoize if there are no children and the props can be memoized
                    // Implementing memoize is unsafe and done automatically with the props trait
                    unsafe {
                        let real_other: &P = &*(other.bump_props as *const _ as *const P);
                        props.memoize(real_other)
                    }
                } else {
                    false
                }
            }
        });

        let drop_props = {
            // create a closure to drop the props
            let mut has_dropped = false;

            let drop_props: &mut dyn FnMut() = bump.alloc_with(|| {
                move || unsafe {
                    if !has_dropped {
                        let real_other = bump_props as *mut _ as *mut P;
                        let b = BumpBox::from_raw(real_other);
                        std::mem::drop(b);

                        has_dropped = true;
                    } else {
                        panic!("Drop props called twice - this is an internal failure of Dioxus");
                    }
                }
            });

            let drop_props = unsafe { BumpBox::from_raw(drop_props) };

            RefCell::new(Some(drop_props))
        };

        let key = key.map(|f| self.raw_text(f).0);

        let caller: &'a mut dyn Fn(&'a ScopeState) -> Element =
            bump.alloc(move |scope: &ScopeState| -> Element {
                let props: &'_ P = unsafe { &*(bump_props as *const P) };
                component(Scope { scope, props })
            });

        let can_memoize = P::IS_STATIC;

        VNode::Component(bump.alloc(VComponent {
            user_fc,
            comparator: Some(comparator),
            bump_props,
            caller,
            key,
            can_memoize,
            drop_props,
            associated_scope: Cell::new(None),
            _hard_allocation: Cell::new(None),
        }))
    }

    pub fn listener(self, event: &'static str, callback: EventHandler<'a>) -> Listener<'a> {
        Listener {
            event,
            mounted_node: Cell::new(None),
            callback,
        }
    }

    pub fn fragment_root<'b, 'c>(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            nodes.push(VNode::Placeholder(self.bump.alloc(VPlaceholder {
                dom_id: empty_cell(),
            })));
        }

        VNode::Fragment(VFragment {
            children: nodes.into_bump_slice(),
            key: None,
        })
    }

    pub fn fragment_from_iter<'b, 'c>(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            nodes.push(VNode::Placeholder(self.bump.alloc(VPlaceholder {
                dom_id: empty_cell(),
            })));
        }

        let children = nodes.into_bump_slice();

        if cfg!(debug_assertions) && children.len() > 1 && children.last().unwrap().key().is_none()
        {
            // todo: make the backtrace prettier or remove it altogether
            log::error!(
                r#"
                Warning: Each child in an array or iterator should have a unique "key" prop.
                Not providing a key will lead to poor performance with lists.
                See docs.rs/dioxus for more information.
                -------------
                {:?}
                "#,
                backtrace::Backtrace::new()
            );
        }

        VNode::Fragment(VFragment {
            children,
            key: None,
        })
    }

    // this isn't quite feasible yet
    // I think we need some form of interior mutability or state on nodefactory that stores which subtree was created
    pub fn create_children(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a>>,
    ) -> Element<'a> {
        let bump = self.bump;
        let mut nodes = bumpalo::collections::Vec::new_in(bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            nodes.push(VNode::Placeholder(bump.alloc(VPlaceholder {
                dom_id: empty_cell(),
            })));
        }

        let children = nodes.into_bump_slice();

        // TODO
        // We need a dedicated path in the rsx! macro that will trigger the "you need keys" warning

        Some(VNode::Fragment(VFragment {
            children,
            key: None,
        }))
    }
}

impl Debug for NodeFactory<'_> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
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
/// ```rust, ignore
/// impl IntoIterator<Item = impl IntoVNode<'a>>
/// ```
///
/// As such, all node creation must go through the factory, which is only available in the component context.
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

// TODO: do we even need this? It almost seems better not to
// // For the case where a rendered VNode is passed into the rsx! macro through curly braces
impl<'a> IntoVNode<'a> for VNode<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        self
    }
}

// Conveniently, we also support "null" (nothing) passed in
impl IntoVNode<'_> for () {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.fragment_from_iter(None as Option<VNode>)
    }
}

// Conveniently, we also support "None"
impl IntoVNode<'_> for Option<()> {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.fragment_from_iter(None as Option<VNode>)
    }
}

impl<'a> IntoVNode<'a> for Option<VNode<'a>> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        self.unwrap_or_else(|| cx.fragment_from_iter(None as Option<VNode>))
    }
}

impl<'a> IntoVNode<'a> for Option<LazyNodes<'a, '_>> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        match self {
            Some(lazy) => lazy.call(cx),
            None => VNode::Fragment(VFragment {
                children: &[],
                key: None,
            }),
        }
    }
}

impl<'a, 'b> IntoVNode<'a> for LazyNodes<'a, 'b> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        self.call(cx)
    }
}

impl IntoVNode<'_> for &'static str {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.static_text(self)
    }
}

impl IntoVNode<'_> for Arguments<'_> {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.text(self)
    }
}

// called cx.render from a helper function
impl IntoVNode<'_> for Option<VPortal> {
    fn into_vnode(self, _cx: NodeFactory) -> VNode {
        match self {
            Some(node) => VNode::Portal(node),
            None => {
                todo!()
            }
        }
    }
}

// essentially passing elements through props
// just build a new element in place
impl IntoVNode<'_> for &Option<VPortal> {
    fn into_vnode(self, _cx: NodeFactory) -> VNode {
        match self {
            Some(node) => VNode::Portal(VPortal {
                link_idx: node.link_idx.clone(),
                scope_id: node.scope_id.clone(),
                node: node.node,
            }),
            None => {
                //
                todo!()
            }
        }
    }
}

impl IntoVNode<'_> for VPortal {
    fn into_vnode(self, _cx: NodeFactory) -> VNode {
        VNode::Portal(self)
    }
}

impl IntoVNode<'_> for &VPortal {
    fn into_vnode(self, _cx: NodeFactory) -> VNode {
        VNode::Portal(VPortal {
            link_idx: self.link_idx.clone(),
            scope_id: self.scope_id.clone(),
            node: self.node,
        })
    }
}
