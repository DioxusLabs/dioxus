//! Virtual Node Support
//!
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers. These VNodes should be *very*
//! cheap and *very* fast to construct - building a full tree should be quick.

use crate::{
    innerlude::{ComponentPtr, Element, Properties, Scope, ScopeId, ScopeState},
    lazynodes::LazyNodes,
    AnyEvent, Component,
};
use bumpalo::{boxed::Box as BumpBox, Bump};
use std::{
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug, Formatter},
};

/// A composable "VirtualNode" to declare a User Interface in the Dioxus VirtualDOM.
///
/// VNodes are designed to be lightweight and used with with a bump allocator. To create a VNode, you can use either of:
///
/// - the `rsx!` macro
/// - the [`NodeFactory`] API
pub enum VNode<'src> {
    /// Text VNodes are simply bump-allocated (or static) string slices
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
    ///         style: { background_color: "red" },
    ///         "hello"
    ///     }
    /// });
    ///
    /// if let VNode::Element(velement) = node {
    ///     assert_eq!(velement.tag_name, "div");
    ///     assert_eq!(velement.namespace, None);
    ///     assert_eq!(velement.key, Some("a"));
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
    Fragment(&'src VFragment<'src>),

    /// Component nodes represent a mounted component with props, children, and a key.
    ///
    /// # Example
    ///
    /// ```rust, ignore
    /// fn Example(cx: Scope) -> Element {
    ///     ...
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
            VNode::Text(el) => el.id.get(),
            VNode::Element(el) => el.id.get(),
            VNode::Placeholder(el) => el.id.get(),
            VNode::Fragment(_) => None,
            VNode::Component(_) => None,
        }
    }

    // Create an "owned" version of the vnode.
    pub(crate) fn decouple(&self) -> VNode<'src> {
        match *self {
            VNode::Text(t) => VNode::Text(t),
            VNode::Element(e) => VNode::Element(e),
            VNode::Component(c) => VNode::Component(c),
            VNode::Placeholder(a) => VNode::Placeholder(a),
            VNode::Fragment(f) => VNode::Fragment(f),
        }
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self {
            VNode::Element(el) => s
                .debug_struct("VNode::Element")
                .field("name", &el.tag)
                .field("key", &el.key)
                .field("attrs", &el.attributes)
                .field("children", &el.children)
                .field("id", &el.id)
                .finish(),
            VNode::Text(t) => s
                .debug_struct("VNode::Text")
                .field("text", &t.text)
                .field("id", &t.id)
                .finish(),
            VNode::Placeholder(t) => s
                .debug_struct("VNode::Placholder")
                .field("id", &t.id)
                .finish(),
            VNode::Fragment(frag) => s
                .debug_struct("VNode::Fragment")
                .field("children", &frag.children)
                .finish(),
            VNode::Component(comp) => s
                .debug_struct("VNode::Component")
                .field("name", &comp.fn_name)
                .field("fnptr", &comp.user_fc)
                .field("key", &comp.key)
                .field("scope", &comp.scope)
                .finish(),
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
    /// Convertt the ElementId to a `u64`.
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

fn empty_cell() -> Cell<Option<ElementId>> {
    Cell::new(None)
}

/// A placeholder node only generated when Fragments don't have any children.
pub struct VPlaceholder {
    /// The [`ElementId`] of the placeholder.
    pub id: Cell<Option<ElementId>>,
}

/// A bump-allocated string slice and metadata.
pub struct VText<'src> {
    /// The [`ElementId`] of the VText.
    pub id: Cell<Option<ElementId>>,

    /// The text of the VText.
    pub text: &'src str,

    /// An indiciation if this VText can be ignored during diffing
    /// Is usually only when there are no strings to be formatted (so the text is &'static str)
    pub is_static: bool,
}

/// A list of VNodes with no single root.
pub struct VFragment<'src> {
    /// The key of the fragment to be used during keyed diffing.
    pub key: Option<&'src str>,

    /// Fragments can never have zero children. Enforced by NodeFactory.
    ///
    /// You *can* make a fragment with no children, but it's not a valid fragment and your VDom will panic.
    pub children: &'src [VNode<'src>],
}

/// An element like a "div" with children, listeners, and attributes.
pub struct VElement<'a> {
    /// The [`ElementId`] of the VText.
    pub id: Cell<Option<ElementId>>,

    /// The key of the element to be used during keyed diffing.
    pub key: Option<&'a str>,

    /// The tag name of the element.
    ///
    /// IE "div"
    pub tag: &'static str,

    /// The namespace of the VElement
    ///
    /// IE "svg"
    pub namespace: Option<&'static str>,

    /// The parent of the Element (if any).
    ///
    /// Used when bubbling events
    pub parent: Cell<Option<ElementId>>,

    /// The Listeners of the VElement.
    pub listeners: &'a [Listener<'a>],

    /// The attributes of the VElement.
    pub attributes: &'a [Attribute<'a>],

    /// The children of the VElement.
    pub children: &'a [VNode<'a>],
}

impl Debug for VElement<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VElement")
            .field("tag_name", &self.tag)
            .field("namespace", &self.namespace)
            .field("key", &self.key)
            .field("id", &self.id)
            .field("parent", &self.parent)
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
    /// The tag name of the element.
    const TAG_NAME: &'static str;

    /// The namespace of the element.
    const NAME_SPACE: Option<&'static str>;

    /// The tag name of the element.
    #[inline]
    fn tag_name(&self) -> &'static str {
        Self::TAG_NAME
    }

    /// The namespace of the element.
    #[inline]
    fn namespace(&self) -> Option<&'static str> {
        Self::NAME_SPACE
    }
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    /// The name of the attribute.
    pub name: &'static str,

    /// The value of the attribute.
    pub value: &'a str,

    /// An indication if this attribute can be ignored during diffing
    ///
    /// Usually only when there are no strings to be formatted (so the value is &'static str)
    pub is_static: bool,

    /// An indication of we should always try and set the attribute.
    /// Used in controlled components to ensure changes are propagated.
    pub is_volatile: bool,

    /// The namespace of the attribute.
    ///
    /// Doesn't exist in the html spec.
    /// Used in Dioxus to denote "style" tags and other attribute groups.
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
    pub(crate) callback: InternalHandler<'bump>,
}

pub type InternalHandler<'bump> = &'bump RefCell<Option<InternalListenerCallback<'bump>>>;
type InternalListenerCallback<'bump> = BumpBox<'bump, dyn FnMut(AnyEvent) + 'bump>;

type ExternalListenerCallback<'bump, T> = BumpBox<'bump, dyn FnMut(T) + 'bump>;

/// The callback type generated by the `rsx!` macro when an `on` field is specified for components.
///
/// This makes it possible to pass `move |evt| {}` style closures into components as property fields.
///
///
/// # Example
///
/// ```rust, ignore
///
/// rsx!{
///     MyComponent { onclick: move |evt| log::info!("clicked"), }
/// }
///
/// #[derive(Props)]
/// struct MyProps<'a> {
///     onclick: EventHandler<'a, MouseEvent>,
/// }
///
/// fn MyComponent(cx: Scope<'a, MyProps<'a>>) -> Element {
///     cx.render(rsx!{
///         button {
///             onclick: move |evt| cx.props.onclick.call(evt),
///         }
///     })
/// }
///
/// ```
pub struct EventHandler<'bump, T = ()> {
    /// The (optional) callback that the user specified
    /// Uses a `RefCell` to allow for interior mutability, and FnMut closures.
    pub callback: RefCell<Option<ExternalListenerCallback<'bump, T>>>,
}

impl<'a, T> Default for EventHandler<'a, T> {
    fn default() -> Self {
        Self {
            callback: RefCell::new(None),
        }
    }
}

impl<T> EventHandler<'_, T> {
    /// Call this event handler with the appropriate event type
    pub fn call(&self, event: T) {
        if let Some(callback) = self.callback.borrow_mut().as_mut() {
            callback(event);
        }
    }

    /// Forcibly drop the internal handler callback, releasing memory
    pub fn release(&self) {
        self.callback.replace(None);
    }
}

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub struct VComponent<'src> {
    /// The key of the component to be used during keyed diffing.
    pub key: Option<&'src str>,

    /// The ID of the component.
    /// Will not be assigned until after the component has been initialized.
    pub scope: Cell<Option<ScopeId>>,

    /// An indication if the component is static (can be memozied)
    pub can_memoize: bool,

    /// The function pointer to the component's render function.
    pub user_fc: ComponentPtr,

    /// The actual name of the component.
    pub fn_name: &'static str,

    /// The props of the component.
    pub props: RefCell<Option<Box<dyn AnyProps + 'src>>>,
}

pub(crate) struct VComponentProps<P> {
    pub render_fn: Component<P>,
    pub memo: unsafe fn(&P, &P) -> bool,
    pub props: P,
}

pub trait AnyProps {
    fn as_ptr(&self) -> *const ();
    fn render<'a>(&'a self, bump: &'a ScopeState) -> Element<'a>;
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool;
}

impl<P> AnyProps for VComponentProps<P> {
    fn as_ptr(&self) -> *const () {
        &self.props as *const _ as *const ()
    }

    // Safety:
    // this will downcast the other ptr as our swallowed type!
    // you *must* make this check *before* calling this method
    // if your functions are not the same, then you will downcast a pointer into a different type (UB)
    unsafe fn memoize(&self, other: &dyn AnyProps) -> bool {
        let real_other: &P = &*(other.as_ptr() as *const _ as *const P);
        let real_us: &P = &*(self.as_ptr() as *const _ as *const P);
        (self.memo)(real_us, real_other)
    }

    fn render<'a>(&'a self, scope: &'a ScopeState) -> Element<'a> {
        let props = unsafe { std::mem::transmute::<&P, &P>(&self.props) };
        (self.render_fn)(Scope { scope, props })
    }
}

/// This struct provides an ergonomic API to quickly build VNodes.
///
/// NodeFactory is used to build VNodes in the component's memory space.
/// This struct adds metadata to the final VNode about listeners, attributes, and children
#[derive(Copy, Clone)]
pub struct NodeFactory<'a> {
    pub(crate) scope: &'a ScopeState,
    pub(crate) bump: &'a Bump,
}

impl<'a> NodeFactory<'a> {
    /// Create a new [`NodeFactory`] from a [`Scope`] or [`ScopeState`]
    pub fn new(scope: &'a ScopeState) -> NodeFactory<'a> {
        NodeFactory {
            scope,
            bump: &scope.wip_frame().bump,
        }
    }

    /// Get the custom allocator for this component
    #[inline]
    pub fn bump(&self) -> &'a bumpalo::Bump {
        self.bump
    }

    /// Directly pass in text blocks without the need to use the format_args macro.
    pub fn static_text(&self, text: &'static str) -> VNode<'a> {
        VNode::Text(self.bump.alloc(VText {
            id: empty_cell(),
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
            id: empty_cell(),
        }))
    }

    /// Create a new [`VNode::VElement`]
    pub fn element(
        &self,
        el: impl DioxusElement,
        listeners: &'a [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [VNode<'a>],
        key: Option<Arguments>,
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

    /// Create a new [`VNode::VElement`] without the trait bound
    ///
    /// IE pass in "div" instead of `div`
    pub fn raw_element(
        &self,
        tag_name: &'static str,
        namespace: Option<&'static str>,
        listeners: &'a [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [VNode<'a>],
        key: Option<Arguments>,
    ) -> VNode<'a> {
        let key = key.map(|f| self.raw_text(f).0);

        let mut items = self.scope.items.borrow_mut();
        for listener in listeners {
            let long_listener = unsafe { std::mem::transmute(listener) };
            items.listeners.push(long_listener);
        }

        VNode::Element(self.bump.alloc(VElement {
            tag: tag_name,
            key,
            namespace,
            listeners,
            attributes,
            children,
            id: empty_cell(),
            parent: empty_cell(),
        }))
    }

    /// Create a new [`Attribute`]
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

    /// Create a new [`VNode::VComponent`]
    pub fn component<P>(
        &self,
        component: fn(Scope<'a, P>) -> Element,
        props: P,
        key: Option<Arguments>,
        fn_name: &'static str,
    ) -> VNode<'a>
    where
        P: Properties + 'a,
    {
        let vcomp = self.bump.alloc(VComponent {
            key: key.map(|f| self.raw_text(f).0),
            scope: Default::default(),
            can_memoize: P::IS_STATIC,
            user_fc: component as ComponentPtr,
            fn_name,
            props: RefCell::new(Some(Box::new(VComponentProps {
                props,
                memo: P::memoize, // smuggle the memoization function across borders

                // i'm sorry but I just need to bludgeon the lifetimes into place here
                // this is safe because we're managing all lifetimes to originate from previous calls
                // the intricacies of Rust's lifetime system make it difficult to properly express
                // the transformation from this specific lifetime to the for<'a> lifetime
                render_fn: unsafe { std::mem::transmute(component) },
            }))),
        });

        if !P::IS_STATIC {
            let vcomp = &*vcomp;
            let vcomp = unsafe { std::mem::transmute(vcomp) };
            self.scope.items.borrow_mut().borrowed_props.push(vcomp);
        }

        VNode::Component(vcomp)
    }

    /// Create a new [`Listener`]
    pub fn listener(self, event: &'static str, callback: InternalHandler<'a>) -> Listener<'a> {
        Listener {
            event,
            mounted_node: Cell::new(None),
            callback,
        }
    }

    /// Create a new [`VNode::VFragment`] from a root of the rsx! call
    pub fn fragment_root<'b, 'c>(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            VNode::Placeholder(self.bump.alloc(VPlaceholder { id: empty_cell() }))
        } else {
            VNode::Fragment(self.bump.alloc(VFragment {
                children: nodes.into_bump_slice(),
                key: None,
            }))
        }
    }

    /// Create a new [`VNode::VFragment`] from any iterator
    pub fn fragment_from_iter<'b, 'c>(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            VNode::Placeholder(self.bump.alloc(VPlaceholder { id: empty_cell() }))
        } else {
            let children = nodes.into_bump_slice();

            if cfg!(debug_assertions)
                && children.len() > 1
                && children.last().unwrap().key().is_none()
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

            VNode::Fragment(self.bump.alloc(VFragment {
                children,
                key: None,
            }))
        }
    }

    /// Create a new [`VNode`] from any iterator of children
    pub fn create_children(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a>>,
    ) -> Element<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            Some(VNode::Placeholder(
                self.bump.alloc(VPlaceholder { id: empty_cell() }),
            ))
        } else {
            let children = nodes.into_bump_slice();

            Some(VNode::Fragment(self.bump.alloc(VFragment {
                children,
                key: None,
            })))
        }
    }

    /// Create a new [`EventHandler`] from an [`FnMut`]
    pub fn event_handler<T>(self, f: impl FnMut(T) + 'a) -> EventHandler<'a, T> {
        let handler: &mut dyn FnMut(T) = self.bump.alloc(f);
        let caller = unsafe { BumpBox::from_raw(handler as *mut dyn FnMut(T)) };
        let callback = RefCell::new(Some(caller));
        EventHandler { callback }
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
    /// Convert this into a [`VNode`], using the [`NodeFactory`] as a source of allocation
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
            None => VNode::Placeholder(cx.bump.alloc(VPlaceholder { id: empty_cell() })),
        }
    }
}

impl<'a, 'b> IntoIterator for LazyNodes<'a, 'b> {
    type Item = LazyNodes<'a, 'b>;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

impl<'a, 'b> IntoVNode<'a> for LazyNodes<'a, 'b> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        self.call(cx)
    }
}

impl<'b> IntoVNode<'_> for &'b str {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.text(format_args!("{}", self))
    }
}

impl IntoVNode<'_> for String {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.text(format_args!("{}", self))
    }
}

impl IntoVNode<'_> for Arguments<'_> {
    fn into_vnode(self, cx: NodeFactory) -> VNode {
        cx.text(self)
    }
}

impl<'a> IntoVNode<'a> for &Option<VNode<'a>> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        self.as_ref()
            .map(|f| f.into_vnode(cx))
            .unwrap_or_else(|| cx.fragment_from_iter(None as Option<VNode>))
    }
}

impl<'a> IntoVNode<'a> for &VNode<'a> {
    fn into_vnode(self, _cx: NodeFactory<'a>) -> VNode<'a> {
        // borrowed nodes are strange
        self.decouple()
    }
}
