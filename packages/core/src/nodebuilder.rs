//! Helpers for building virtual DOM VNodes.

use std::{
    any::Any,
    borrow::BorrowMut,
    cell::{Cell, RefCell},
    fmt::Arguments,
    intrinsics::transmute,
    u128,
};

use bumpalo::Bump;

use crate::{
    events::VirtualEvent,
    innerlude::{Properties, VComponent, VText, FC},
    nodes::{Attribute, Listener, NodeKey, VNode},
    prelude::{VElement, VFragment},
    virtual_dom::{RealDomNode, Scope},
};

/// A virtual DOM element builder.
///
/// Typically constructed with element-specific constructors, eg the `div`
/// function for building `<div>` elements or the `button` function for building
/// `<button>` elements.
#[derive(Debug)]
pub struct ElementBuilder<'a, 'b, Listeners, Attributes, Children>
where
    Listeners: 'a + AsRef<[Listener<'a>]>,
    Attributes: 'a + AsRef<[Attribute<'a>]>,
    Children: 'a + AsRef<[VNode<'a>]>,
{
    cx: &'b NodeFactory<'a>,
    key: NodeKey<'a>,
    tag_name: &'static str,
    listeners: Listeners,
    attributes: Attributes,
    children: Children,
    namespace: Option<&'static str>,
}

impl<'a, 'b>
    ElementBuilder<
        'a,
        'b,
        bumpalo::collections::Vec<'a, Listener<'a>>,
        bumpalo::collections::Vec<'a, Attribute<'a>>,
        bumpalo::collections::Vec<'a, VNode<'a>>,
    >
{
    /// Create a new `ElementBuilder` for an element with the given tag name.
    ///
    /// In general, only use this constructor if the tag is dynamic (i.e. you
    /// might build a `<div>` or you might build a `<span>` and you don't know
    /// until runtime). Prefer using the tag-specific constructors instead:
    /// `div(bump)` or `span(bump)`, etc.
    ///
    /// # Example
    ///
    /// ```
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// let tag_name = if flip_coin() {
    ///     "div"
    /// } else {
    ///     "span"
    /// };
    ///
    /// let my_element_builder = ElementBuilder::new(&b, tag_name);
    /// # fn flip_coin() -> bool { true }
    /// ```
    pub fn new(cx: &'b NodeFactory<'a>, tag_name: &'static str) -> Self {
        let bump = cx.bump();
        ElementBuilder {
            cx,
            key: NodeKey::NONE,
            tag_name,
            listeners: bumpalo::collections::Vec::new_in(bump),
            attributes: bumpalo::collections::Vec::new_in(bump),
            children: bumpalo::collections::Vec::new_in(bump),
            namespace: None,
        }
    }
}

impl<'a, 'b, Listeners, Attributes, Children>
    ElementBuilder<'a, 'b, Listeners, Attributes, Children>
where
    Listeners: 'a + AsRef<[Listener<'a>]>,
    Attributes: 'a + AsRef<[Attribute<'a>]>,
    Children: 'a + AsRef<[VNode<'a>]>,
{
    /// Set the listeners for this element.
    ///
    /// You can use this method to customize the backing storage for listeners,
    /// for example to use a fixed-size array instead of the default
    /// dynamically-sized `bumpalo::collections::Vec`.
    ///
    /// Any listeners already added to the builder will be overridden.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// // Create a `<div>` with a fixed-size array of two listeners.
    /// let my_div = div(&b)
    ///     .listeners([
    ///         on(&b, "click", |root, vdom, event| {
    ///             // ...
    ///         }),
    ///         on(&b, "dblclick", |root, vdom, event| {
    ///             // ...
    ///         }),
    ///     ])
    ///     .finish();
    /// ```
    #[inline]
    pub fn listeners<L>(self, listeners: L) -> ElementBuilder<'a, 'b, L, Attributes, Children>
    where
        L: 'a + AsRef<[Listener<'a>]>,
    {
        ElementBuilder {
            cx: self.cx,
            key: self.key,
            tag_name: self.tag_name,
            listeners,
            attributes: self.attributes,
            children: self.children,
            namespace: self.namespace,
        }
    }

    /// Set the attributes for this element.
    ///
    /// You can use this method to customize the backing storage for attributes,
    /// for example to use a fixed-size array instead of the default
    /// dynamically-sized `bumpalo::collections::Vec`.
    ///
    /// Any attributes already added to the builder will be overridden.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump, Attribute};
    ///
    /// let b = Bump::new();
    ///
    /// // Create a `<div>` with a fixed-size array of two attributes.
    /// let my_div = div(&b)
    ///     .attributes([
    ///         attr("id", "my-div"),
    ///         attr("class", "notification"),
    ///     ])
    ///     .finish();
    /// ```
    #[inline]
    pub fn attributes<A>(self, attributes: A) -> ElementBuilder<'a, 'b, Listeners, A, Children>
    where
        A: 'a + AsRef<[Attribute<'a>]>,
    {
        ElementBuilder {
            cx: self.cx,
            key: self.key,
            tag_name: self.tag_name,
            listeners: self.listeners,
            attributes,
            children: self.children,
            namespace: self.namespace,
        }
    }

    /// Set the children for this element.
    ///
    /// You can use this method to customize the backing storage for children,
    /// for example to use a fixed-size array instead of the default
    /// dynamically-sized `bumpalo::collections::Vec`.
    ///
    /// Any children already added to the builder will be overridden.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// // Create a `<div>` with a fixed-size array of two `<span>` children.
    /// let my_div = div(&b)
    ///     .children([
    ///         span(&b).finish(),
    ///         span(&b).finish(),
    ///     ])
    ///     .finish();
    /// ```
    #[inline]
    pub fn children<C>(self, children: C) -> ElementBuilder<'a, 'b, Listeners, Attributes, C>
    where
        C: 'a + AsRef<[VNode<'a>]>,
    {
        ElementBuilder {
            cx: self.cx,
            key: self.key,
            tag_name: self.tag_name,
            listeners: self.listeners,
            attributes: self.attributes,
            children,
            namespace: self.namespace,
        }
    }

    /// Set the namespace for this element.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// // Create a `<td>` tag with an xhtml namespace
    /// let my_td = td(&b)
    ///     .namespace(Some("http://www.w3.org/1999/xhtml"))
    ///     .finish();
    /// ```
    #[inline]
    pub fn namespace(self, namespace: Option<&'static str>) -> Self {
        ElementBuilder {
            cx: self.cx,
            key: self.key,
            tag_name: self.tag_name,
            listeners: self.listeners,
            attributes: self.attributes,
            children: self.children,
            namespace,
        }
    }

    /// Set this element's key.
    ///
    /// When diffing sets of siblings, if an old sibling and new sibling share a
    /// key, then they will always reuse the same physical DOM VNode. This is
    /// important when using CSS animations, web components, third party JS, or
    /// anything else that makes the diffing implementation observable.
    ///
    /// Do not use keys if such a scenario does not apply. Keyed diffing is
    /// generally more expensive than not, since it is putting greater
    /// constraints on the diffing algorithm.
    ///
    /// # Invariants You Must Uphold
    ///
    /// The key may not be `u32::MAX`, which is a reserved key value.
    ///
    /// Keys must be unique among siblings.
    ///
    /// All sibling VNodes must be keyed, or they must all not be keyed. You may
    /// not mix keyed and unkeyed siblings.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// let my_li = li(&b)
    ///     .key(1337)
    ///     .finish();
    /// ```
    #[inline]
    pub fn key(mut self, key: &'a str) -> Self {
        self.key = NodeKey(Some(key));
        self
    }

    pub fn key2(mut self, args: Arguments) -> Self {
        let key = match args.as_str() {
            Some(static_str) => static_str,
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(self.cx.bump());
                s.write_fmt(args).unwrap();
                s.into_bump_str()
            }
        };
        self.key = NodeKey(Some(key));
        self
    }

    /// Create the virtual DOM VNode described by this builder.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump, VNode};
    ///
    /// let b = Bump::new();
    ///
    /// // Start with a builder...
    /// let builder: ElementBuilder<_, _, _> = div(&b);
    ///
    /// // ...and finish it to create a virtual DOM VNode!
    /// let my_div: VNode = builder.finish();
    /// ```
    #[inline]
    pub fn finish(self) -> VNode<'a> {
        let bump = self.cx.bump();

        let children: &'a Children = bump.alloc(self.children);
        let children: &'a [VNode<'a>] = children.as_ref();

        let listeners: &'a Listeners = bump.alloc(self.listeners);
        let listeners: &'a [Listener<'a>] = listeners.as_ref();

        let attributes: &'a Attributes = bump.alloc(self.attributes);
        let attributes: &'a [Attribute<'a>] = attributes.as_ref();

        VNode::element(
            bump,
            self.key,
            self.tag_name,
            listeners,
            attributes,
            children,
            self.namespace,
        )
    }
}

impl<'a, 'b, Attributes, Children>
    ElementBuilder<'a, 'b, bumpalo::collections::Vec<'a, Listener<'a>>, Attributes, Children>
where
    Attributes: 'a + AsRef<[Attribute<'a>]>,
    Children: 'a + AsRef<[VNode<'a>]>,
{
    /// Add a new event listener to this element.
    ///
    /// The `event` string specifies which event will be listened for. The
    /// `callback` function is the function that will be invoked if the
    /// specified event occurs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// // A button that does something when clicked!
    /// let my_button = button(&b)
    ///     .on("click", |event| {
    ///         // ...
    ///     })
    ///     .finish();
    /// ```
    pub fn on(self, event: &'static str, callback: impl Fn(VirtualEvent) + 'a) -> Self {
        let bump = &self.cx.bump();
        let listener = Listener {
            event,
            callback: bump.alloc(callback),
            scope: self.cx.scope_ref.arena_idx,
            mounted_node: bump.alloc(Cell::new(RealDomNode::empty())),
        };
        self.add_listener(listener)
    }

    pub fn add_listener(mut self, listener: Listener<'a>) -> Self {
        self.listeners.push(listener);

        // bump the context id forward
        let id = self.cx.listener_id.get();
        self.cx.listener_id.set(id + 1);

        // Add this listener to the context list
        // This casts the listener to a self-referential pointer
        // This is okay because the bump arena is stable
        self.listeners.last().map(|g| {
            let r = unsafe { std::mem::transmute::<&Listener<'a>, &Listener<'static>>(g) };
            self.cx
                .scope_ref
                .listeners
                .borrow_mut()
                .push((r.mounted_node as *const _, r.callback as *const _));
        });

        self
    }
}

impl<'a, 'b, Listeners, Children>
    ElementBuilder<'a, 'b, Listeners, bumpalo::collections::Vec<'a, Attribute<'a>>, Children>
where
    Listeners: 'a + AsRef<[Listener<'a>]>,
    Children: 'a + AsRef<[VNode<'a>]>,
{
    /// Add a new attribute to this element.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// // Create the `<div id="my-div"/>` element.
    /// let my_div = div(&b).attr("id", "my-div").finish();
    /// ```
    pub fn attr(mut self, name: &'static str, args: std::fmt::Arguments) -> Self {
        let value = match args.as_str() {
            Some(static_str) => static_str,
            None => {
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(self.cx.bump());
                s.write_fmt(args).unwrap();
                s.into_bump_str()
            }
        };

        self.attributes.push(Attribute {
            name,
            value,
            namespace: None,
        });
        self
    }

    /// Conditionally add a "boolean-style" attribute to this element.
    ///
    /// If the `should_add` parameter is true, then adds an attribute with the
    /// given `name` and an empty string value. If the `should_add` parameter is
    /// false, then the attribute is not added.
    ///
    /// This method is useful for attributes whose semantics are defined by
    /// whether or not the attribute is present or not, and whose value is
    /// ignored. Example attributes like this include:
    ///
    /// * `checked`
    /// * `hidden`
    /// * `selected`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    /// use js_sys::Math;
    ///
    /// let b = Bump::new();
    ///
    /// // Create the `<div>` that is randomly hidden 50% of the time.
    /// let my_div = div(&b)
    ///     .bool_attr("hidden", Math::random() >= 0.5)
    ///     .finish();
    /// ```
    pub fn bool_attr(mut self, name: &'static str, should_add: bool) -> Self {
        if should_add {
            self.attributes.push(Attribute {
                name,
                value: "",
                namespace: None,
            });
        }
        self
    }
}

impl<'a, 'b, Listeners, Attributes>
    ElementBuilder<'a, 'b, Listeners, Attributes, bumpalo::collections::Vec<'a, VNode<'a>>>
where
    Listeners: 'a + AsRef<[Listener<'a>]>,
    Attributes: 'a + AsRef<[Attribute<'a>]>,
{
    /// Add a new child to this element.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    /// use js_sys::Math;
    ///
    /// let b = Bump::new();
    ///
    /// // Create `<p><span></span></p>`.
    /// let my_div = p(&b)
    ///     .child(span(&b).finish())
    ///     .finish();
    /// ```
    #[inline]
    pub fn child(mut self, child: VNode<'a>) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple children to this element from an iterator.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dioxus::{builder::*, bumpalo::Bump};
    ///
    /// let b = Bump::new();
    ///
    /// let my_div = p(&b)
    ///     .iter_child((0..10).map(|f| span(&b).finish())
    ///     .finish();
    /// ```    
    pub fn iter_child(mut self, nodes: impl IntoIterator<Item = impl IntoVNode<'a>>) -> Self {
        let len_before = self.children.len();
        for item in nodes {
            let child = item.into_vnode(&self.cx);
            self.children.push(child);
        }
        if cfg!(debug_assertions) {
            if self.children.len() > len_before + 1 {
                if self.children.last().unwrap().key().is_none() {
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
        self
    }
}

impl<'a> IntoIterator for VNode<'a> {
    type Item = VNode<'a>;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}
impl<'a> IntoVNode<'a> for VNode<'a> {
    fn into_vnode(self, cx: &NodeFactory<'a>) -> VNode<'a> {
        self
    }
}

impl<'a> IntoVNode<'a> for &VNode<'a> {
    fn into_vnode(self, cx: &NodeFactory<'a>) -> VNode<'a> {
        // cloning is cheap since vnodes are just references into bump arenas
        self.clone()
    }
}

pub trait IntoVNode<'a> {
    fn into_vnode(self, cx: &NodeFactory<'a>) -> VNode<'a>;
}

pub trait VNodeBuilder<'a, G>: IntoIterator<Item = G>
where
    G: IntoVNode<'a>,
{
}

impl<'a, F> VNodeBuilder<'a, LazyNodes<'a, F>> for LazyNodes<'a, F> where
    F: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a
{
}

// Wrap the the node-builder closure in a concrete type.
// ---
// This is a bit of a hack to implement the IntoVNode trait for closure types.
pub struct LazyNodes<'a, G>
where
    G: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
{
    inner: G,
    _p: std::marker::PhantomData<&'a ()>,
}

impl<'a, G> LazyNodes<'a, G>
where
    G: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
{
    pub fn new(f: G) -> Self {
        Self {
            inner: f,
            _p: std::default::Default::default(),
        }
    }
}

// Cover the cases where nodes are used by macro.
// Likely used directly.
// ---
//  let nodes = rsx!{ ... };
//  rsx! { {nodes } }
impl<'a, G> IntoVNode<'a> for LazyNodes<'a, G>
where
    G: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
{
    fn into_vnode(self, cx: &NodeFactory<'a>) -> VNode<'a> {
        (self.inner)(cx)
    }
}

// Required because anything that enters brackets in the rsx! macro needs to implement IntoIterator
impl<'a, G> IntoIterator for LazyNodes<'a, G>
where
    G: for<'b> FnOnce(&'b NodeFactory<'a>) -> VNode<'a> + 'a,
{
    type Item = Self;
    type IntoIter = std::iter::Once<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self)
    }
}

impl<'a> IntoVNode<'a> for () {
    fn into_vnode(self, cx: &NodeFactory<'a>) -> VNode<'a> {
        todo!();
        VNode::Suspended {
            real: Cell::new(RealDomNode::empty()),
        }
    }
}

impl<'a> IntoVNode<'a> for Option<()> {
    fn into_vnode(self, cx: &NodeFactory<'a>) -> VNode<'a> {
        todo!();
        VNode::Suspended {
            real: Cell::new(RealDomNode::empty()),
        }
    }
}

/// Construct a text VNode.
///
/// This is `dioxus`'s virtual DOM equivalent of `document.createTextVNode`.
///
/// # Example
///
/// ```no_run
/// use dioxus::builder::*;
///
/// let my_text = text("hello, dioxus!");
/// ```
#[inline]
pub fn text<'a>(contents: &'a str) -> VNode<'a> {
    VNode::text(contents)
}

pub fn text2<'a>(contents: bumpalo::collections::String<'a>) -> VNode<'a> {
    let f: &'a str = contents.into_bump_str();
    VNode::text(f)
}

pub fn text3<'a>(bump: &'a bumpalo::Bump, args: std::fmt::Arguments) -> VNode<'a> {
    // This is a cute little optimization
    //
    // We use format_args! on everything. However, not all textnodes have dynamic content, and thus are completely static.
    // we can just short-circuit to the &'static str version instead of having to allocate in the bump arena.
    //
    // In the most general case, this prevents the need for any string allocations for simple code IE:
    // ```
    //  div {"abc"}
    // ```
    VNode::text(raw_text(bump, args))
}

pub fn raw_text<'a>(bump: &'a bumpalo::Bump, args: std::fmt::Arguments) -> &'a str {
    match args.as_str() {
        Some(static_str) => static_str,
        None => {
            use bumpalo::core_alloc::fmt::Write;
            let mut s = bumpalo::collections::String::new_in(bump);
            s.write_fmt(args).unwrap();
            s.into_bump_str()
        }
    }
}

/// Construct an attribute for an element.
///
/// # Example
///
/// This example creates the `id="my-id"` for some element like `<div
/// id="my-id"/>`.
///
/// ```no_run
/// use dioxus::builder::*;
///
/// let my_id_attr = attr("id", "my-id");
/// ```
pub fn attr<'a>(name: &'static str, value: &'a str) -> Attribute<'a> {
    Attribute {
        name,
        value,
        namespace: None,
    }
}

pub fn virtual_child<'a, T: Properties + 'a>(
    cx: &NodeFactory<'a>,
    f: FC<T>,
    props: T,
    key: Option<&'a str>, // key: NodeKey<'a>,
    children: &'a [VNode<'a>],
) -> VNode<'a> {
    // currently concerned about if props have a custom drop implementation
    // might override it with the props macro
    // todo!()
    VNode::Component(
        cx.bump()
            .alloc(crate::nodes::VComponent::new(cx, f, props, key, children)),
        // cx.bump()
        //     .alloc(crate::nodes::VComponent::new(f, props, key)),
    )
}

pub fn vfragment<'a>(
    cx: &NodeFactory<'a>,
    key: Option<&'a str>, // key: NodeKey<'a>,
    children: &'a [VNode<'a>],
) -> VNode<'a> {
    VNode::Fragment(cx.bump().alloc(VFragment::new(key, children)))
}

pub struct ChildrenList<'a, 'b> {
    cx: &'b NodeFactory<'a>,
    children: bumpalo::collections::Vec<'a, VNode<'a>>,
}

impl<'a, 'b> ChildrenList<'a, 'b> {
    pub fn new(cx: &'b NodeFactory<'a>) -> Self {
        Self {
            cx,
            children: bumpalo::collections::Vec::new_in(cx.bump()),
        }
    }

    pub fn add_child(mut self, nodes: impl IntoIterator<Item = impl IntoVNode<'a>>) -> Self {
        for item in nodes {
            let child = item.into_vnode(&self.cx);
            self.children.push(child);
        }
        self
    }

    pub fn finish(self) -> &'a [VNode<'a>] {
        self.children.into_bump_slice()
    }
}

/// This struct provides an ergonomic API to quickly build VNodes.
///
/// NodeFactory is used to build VNodes in the component's memory space.
/// This struct adds metadata to the final VNode about listeners, attributes, and children
#[derive(Clone)]
pub struct NodeFactory<'a> {
    pub scope_ref: &'a Scope,
    pub listener_id: Cell<usize>,
}

impl<'a> NodeFactory<'a> {
    #[inline]
    pub fn bump(&self) -> &'a bumpalo::Bump {
        &self.scope_ref.cur_frame().bump
    }

    /// Create some text that's allocated along with the other vnodes
    pub fn text(&self, args: Arguments) -> VNode<'a> {
        text3(self.bump(), args)
    }

    /// Create an element builder
    pub fn element<'b>(
        &'b self,
        tag: impl DioxusElement,
    ) -> ElementBuilder<
        'a,
        'b,
        bumpalo::collections::Vec<'a, Listener<'a>>,
        bumpalo::collections::Vec<'a, Attribute<'a>>,
        bumpalo::collections::Vec<'a, VNode<'a>>,
    > {
        ElementBuilder::new(self, tag.tag_name())
    }

    pub fn attr(
        &self,
        name: &'static str,
        val: Arguments<'a>,
        namespace: Option<&'static str>,
    ) -> Attribute<'a> {
        let value = raw_text(self.bump(), val);
        Attribute {
            name,
            value,
            namespace,
        }
    }

    pub fn child_list(&self) -> ChildrenList {
        ChildrenList::new(&self)
    }

    pub fn fragment_builder<'b>(
        &'b self,
        key: Option<&'a str>,
        builder: impl FnOnce(ChildrenList<'a, 'b>) -> &'a [VNode<'a>],
    ) -> VNode<'a> {
        self.fragment(builder(ChildrenList::new(&self)), key)
    }

    pub fn fragment(&self, children: &'a [VNode<'a>], key: Option<&'a str>) -> VNode<'a> {
        VNode::Fragment(self.bump().alloc(VFragment {
            children,
            key: NodeKey::new_opt(key),
        }))
    }

    pub fn fragment_from_iter(
        &self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a>>,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump());
        for node in node_iter.into_iter() {
            nodes.push(node.into_vnode(&self));
        }
        VNode::Fragment(
            self.bump()
                .alloc(VFragment::new(None, nodes.into_bump_slice())),
        )
    }
}

use std::fmt::Debug;
impl Debug for NodeFactory<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub trait DioxusElement {
    const TAG_NAME: &'static str;
    const NAME_SPACE: Option<&'static str>;
    fn tag_name(&self) -> &'static str {
        Self::TAG_NAME
    }
}
