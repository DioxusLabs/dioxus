//! Virtual Node Support
//!
//! VNodes represent lazily-constructed VDom trees that support diffing and event handlers. These VNodes should be *very*
//! cheap and *very* fast to construct - building a full tree should be quick.

use crate::innerlude::{
    empty_cell, Context, Element, ElementId, Properties, ScopeId, ScopeInner, SuspendedContext, FC,
};
use bumpalo::{boxed::Box as BumpBox, Bump};
use std::{
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug, Formatter},
};

/// A composable "VirtualNode" to declare a User Interface in the Dioxus VirtualDOM.
///
/// VNodes are designed to be lightweight and used with with a bump allocator. To create a VNode, you can use either of:
/// - the [`rsx`] macro
/// - the [`html`] macro
/// - the [`NodeFactory`] API
pub enum VNode<'src> {
    /// Text VNodes simply bump-allocated (or static) string slices
    ///
    /// # Example
    ///
    /// ```
    /// let node = cx.render(rsx!{ "hello" }).unwrap();
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
    /// ```rust
    /// let node = cx.render(rsx!{
    ///     div {
    ///         key: "a",
    ///         onclick: |e| log::info!("clicked"),
    ///         hidden: "true",
    ///         style: { background_color: "red" }
    ///         "hello"
    ///     }
    /// }).unwrap();
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
    /// ```rust
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
    /// ```rust
    /// fn Example(cx: Context<()>) -> DomTree {
    ///     todo!()
    /// }
    ///
    /// let node = cx.render(rsx!{
    ///     Example {}
    /// }).unwrap();
    ///
    /// if let VNode::Component(vcomp) = node {
    ///     assert_eq!(vcomp.user_fc, Example as *const ());
    /// }
    /// ```
    Component(&'src VComponent<'src>),

    /// Suspended VNodes represent chunks of the UI tree that are not yet ready to be displayed.
    ///
    /// These nodes currently can only be constructed via the [`use_suspense`] hook.
    ///
    /// # Example
    ///
    /// ```rust
    /// rsx!{
    /// }
    /// ```
    Suspended(&'src VSuspended<'src>),

    /// Anchors are a type of placeholder VNode used when fragments don't contain any children.
    ///
    /// Anchors cannot be directly constructed via public APIs.
    ///
    /// # Example
    ///
    /// ```rust
    /// let node = cx.render(rsx! ( Fragment {} )).unwrap();
    /// if let VNode::Fragment(frag) = node {
    ///     let root = &frag.children[0];
    ///     assert_eq!(root, VNode::Anchor);
    /// }
    /// ```
    Anchor(&'src VAnchor),
}

impl<'src> VNode<'src> {
    /// Get the VNode's "key" used in the keyed diffing algorithm.
    pub fn key(&self) -> Option<&'src str> {
        match &self {
            VNode::Element(el) => el.key,
            VNode::Component(c) => c.key,
            VNode::Fragment(f) => f.key,
            VNode::Text(_t) => None,
            VNode::Suspended(_s) => None,
            VNode::Anchor(_f) => None,
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
            VNode::Anchor(el) => el.dom_id.get(),
            VNode::Suspended(el) => el.dom_id.get(),
            VNode::Fragment(_) => None,
            VNode::Component(_) => None,
        }
    }
}

/// A placeholder node only generated when Fragments don't have any children.
pub struct VAnchor {
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

    pub is_static: bool,
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
/// ```rust
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

    #[allow(clippy::type_complexity)]
    /// The actual callback that the user specified
    pub(crate) callback:
        RefCell<Option<BumpBox<'bump, dyn FnMut(Box<dyn std::any::Any + Send>) + 'bump>>>,
}

/// Virtual Components for custom user-defined components
/// Only supports the functional syntax
pub struct VComponent<'src> {
    pub key: Option<&'src str>,

    pub associated_scope: Cell<Option<ScopeId>>,

    pub is_static: bool,

    // Function pointer to the FC that was used to generate this component
    pub user_fc: *const (),

    pub(crate) caller: &'src dyn for<'b> Fn(&'b ScopeInner) -> Element<'b>,

    pub(crate) children: &'src [VNode<'src>],

    pub(crate) comparator: Option<&'src dyn Fn(&VComponent) -> bool>,

    pub(crate) drop_props: RefCell<Option<BumpBox<'src, dyn FnMut()>>>,

    pub(crate) can_memoize: bool,

    // Raw pointer into the bump arena for the props of the component
    pub(crate) raw_props: *const (),
}

pub struct VSuspended<'a> {
    pub task_id: u64,
    pub dom_id: Cell<Option<ElementId>>,

    #[allow(clippy::type_complexity)]
    pub callback: RefCell<Option<BumpBox<'a, dyn FnMut(SuspendedContext<'a>) -> Element<'a>>>>,
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

    pub(crate) fn unstable_place_holder(&self) -> VNode {
        VNode::Text(self.bump.alloc(VText {
            text: "",
            dom_id: empty_cell(),
            is_static: true,
        }))
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
                let mut str_buf = bumpalo::collections::String::new_in(self.bump());
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
        let listeners: &'a L = self.bump().alloc(listeners);
        let listeners = listeners.as_ref();

        let attributes: &'a A = self.bump().alloc(attributes);
        let attributes = attributes.as_ref();

        let children: &'a V = self.bump().alloc(children);
        let children = children.as_ref();

        let key = key.map(|f| self.raw_text(f).0);

        VNode::Element(self.bump().alloc(VElement {
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

    pub fn component<P, V>(
        &self,
        component: FC<P>,
        props: P,
        key: Option<Arguments>,
        children: V,
    ) -> VNode<'a>
    where
        P: Properties + 'a,
        V: AsRef<[VNode<'a>]> + 'a,
        // V: 'a + AsRef<[VNode<'a>]>,
    {
        let bump = self.bump();
        let children: &'a V = bump.alloc(children);
        let children = children.as_ref();
        let props = bump.alloc(props);
        let raw_props = props as *mut P as *mut ();
        let user_fc = component as *const ();

        let comparator: Option<&dyn Fn(&VComponent) -> bool> = Some(bump.alloc_with(|| {
            move |other: &VComponent| {
                if user_fc == other.user_fc {
                    // Safety
                    // - We guarantee that FC<P> is the same by function pointer
                    // - Because FC<P> is the same, then P must be the same (even with generics)
                    // - Non-static P are autoderived to memoize as false
                    // - This comparator is only called on a corresponding set of bumpframes
                    let props_memoized = unsafe {
                        let real_other: &P = &*(other.raw_props as *const _ as *const P);
                        props.memoize(real_other)
                    };

                    log::debug!("comparing props...");

                    // It's only okay to memoize if there are no children and the props can be memoized
                    // Implementing memoize is unsafe and done automatically with the props trait
                    matches!((props_memoized, children.is_empty()), (true, true))
                } else {
                    false
                }
            }
        }));

        let drop_props = {
            // create a closure to drop the props
            let mut has_dropped = false;

            let drop_props: &mut dyn FnMut() = bump.alloc_with(|| {
                move || unsafe {
                    log::debug!("dropping props!");
                    if !has_dropped {
                        let real_other = raw_props as *mut _ as *mut P;
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

        let is_static = children.is_empty() && P::IS_STATIC && key.is_none();

        let key = key.map(|f| self.raw_text(f).0);

        let caller: &'a mut dyn for<'b> Fn(&'b ScopeInner) -> Element<'b> =
            bump.alloc(move |scope: &ScopeInner| -> Element {
                log::debug!("calling component renderr {:?}", scope.our_arena_idx);
                let props: &'_ P = unsafe { &*(raw_props as *const P) };
                let res: Element = component((Context { scope }, props));
                unsafe { std::mem::transmute(res) }
            });

        let can_memoize = children.is_empty() && P::IS_STATIC;

        VNode::Component(bump.alloc(VComponent {
            user_fc,
            comparator,
            raw_props,
            children,
            caller,
            is_static,
            key,
            can_memoize,
            drop_props,
            associated_scope: Cell::new(None),
        }))
    }

    pub fn fragment_from_iter(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a>>,
    ) -> VNode<'a> {
        let bump = self.bump();
        let mut nodes = bumpalo::collections::Vec::new_in(bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        if nodes.is_empty() {
            nodes.push(VNode::Anchor(bump.alloc(VAnchor {
                dom_id: empty_cell(),
            })));
        }

        let children = nodes.into_bump_slice();

        // TODO
        // We need a dedicated path in the rsx! macro that will trigger the "you need keys" warning
        //
        // if cfg!(debug_assertions) {
        //     if children.len() > 1 {
        //         if children.last().unwrap().key().is_none() {
        //             log::error!(
        //                 r#"
        // Warning: Each child in an array or iterator should have a unique "key" prop.
        // Not providing a key will lead to poor performance with lists.
        // See docs.rs/dioxus for more information.
        // ---
        // To help you identify where this error is coming from, we've generated a backtrace.
        //                         "#,
        //             );
        //         }
        //     }
        // }

        VNode::Fragment(VFragment {
            children,
            key: None,
            is_static: false,
        })
    }

    pub fn annotate_lazy<'z, 'b, F>(
        f: F,
    ) -> Option<Box<dyn FnOnce(NodeFactory<'z>) -> VNode<'z> + 'b>>
    where
        F: FnOnce(NodeFactory<'z>) -> VNode<'z> + 'b,
    {
        Some(Box::new(f))
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
/// As such, all node creation must go through the factory, which is only available in the component context.
/// These strict requirements make it possible to manage lifetimes and state.
pub trait IntoVNode<'a> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a>;
}

/// Child nodes of the parent component.
///
/// # Example
///
/// ```rust
/// let children = cx.children();
/// let first_node = &children[0];
/// rsx!{
///     h1 { {first_node} }
///     p { {&children[1..]} }
/// }
/// ```
///
pub struct ScopeChildren<'a>(pub &'a [VNode<'a>]);

impl Copy for ScopeChildren<'_> {}

impl<'a> Clone for ScopeChildren<'a> {
    fn clone(&self) -> Self {
        ScopeChildren(self.0)
    }
}

impl ScopeChildren<'_> {
    // dangerous method - used to fix the associated lifetime
    pub(crate) unsafe fn extend_lifetime(self) -> ScopeChildren<'static> {
        std::mem::transmute(self)
    }

    // dangerous method - used to fix the associated lifetime
    pub(crate) unsafe fn shorten_lifetime<'a>(self) -> ScopeChildren<'a> {
        std::mem::transmute(self)
    }
}

// impl IntoVNodeList for ScopeChildren<'_> {
//     fn into_vnode_list<'a>(self, _: NodeFactory<'a>) -> &'a [VNode<'a>] {
//         self.0
//     }
// }

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
// impl IntoVNode for VNode<'_> {
//     fn into_vnode<'a>(self, _: NodeFactory<'a>) -> VNode<'a> {
//         self
//     }
// }

// Conveniently, we also support "null" (nothing) passed in
// impl IntoVNode for () {
//     fn into_vnode(self, cx: NodeFactory) -> VNode {
//         cx.fragment_from_iter(None as Option<VNode>)
//     }
// }

// // Conveniently, we also support "None"
// impl IntoVNode for Option<()> {
//     fn into_vnode(self, cx: NodeFactory) -> VNode {
//         cx.fragment_from_iter(None as Option<VNode>)
//     }
// }

// impl IntoVNode for Option<VNode<'_>> {
//     fn into_vnode<'a>(self, cx: NodeFactory<'a>) -> VNode<'a> {
//         match self {
//             Some(n) => n,
//             None => cx.fragment_from_iter(None as Option<VNode>),
//         }
//     }
// }

pub fn annotate_lazy<'a, 'b, F>(f: F) -> Option<Box<dyn FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b>>
where
    F: FnOnce(NodeFactory<'a>) -> VNode<'a> + 'b,
{
    Some(Box::new(f))
}

impl<'a> IntoVNode<'a> for Option<Box<dyn FnOnce(NodeFactory<'a>) -> VNode<'a> + '_>> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        match self {
            Some(n) => {
                //
                // n.into_vnode(cx)
                n(cx)
            }
            None => VNode::Fragment(VFragment {
                children: &[],
                is_static: false,
                key: None,
            }),
        }
    }
}

impl<'a> IntoVNode<'a> for Box<dyn FnOnce(NodeFactory<'a>) -> VNode<'a> + '_> {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        self(cx)
    }
}

// impl<'a, F: FnOnce(NodeFactory<'a>) -> VNode<'a>> IntoVNode<'a> for Option<LazyNodes<'a, F>> {
//     fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
//         match self {
//             Some(n) => n.into_vnode(cx),
//             None => cx.fragment_from_iter(None as Option<VNode>),
//         }
//     }
// }

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

impl Debug for NodeFactory<'_> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Debug for VNode<'_> {
    fn fmt(&self, s: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        match &self {
            VNode::Element(el) => s
                .debug_struct("VElement")
                .field("name", &el.tag_name)
                .field("key", &el.key)
                .finish(),

            VNode::Text(t) => write!(s, "VText {{ text: {} }}", t.text),
            VNode::Anchor(_) => write!(s, "VAnchor"),

            VNode::Fragment(frag) => write!(s, "VFragment {{ children: {:?} }}", frag.children),
            VNode::Suspended { .. } => write!(s, "VSuspended"),
            VNode::Component(comp) => write!(
                s,
                "VComponent {{ fc: {:?}, children: {:?} }}",
                comp.user_fc, comp.children
            ),
        }
    }
}
