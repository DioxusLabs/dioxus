//! Helpers for building virtual DOM VNodes.

use std::ops::Deref;

use crate::{
    context::NodeCtx,
    events::VirtualEvent,
    innerlude::VComponent,
    nodes::{Attribute, Listener, NodeKey, VNode},
    prelude::VElement,
};

use bumpalo::format;
use bumpalo::Bump;

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
    ctx: &'b NodeCtx<'a>,
    key: NodeKey,
    tag_name: &'a str,
    listeners: Listeners,
    attributes: Attributes,
    children: Children,
    namespace: Option<&'a str>,
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
    pub fn new(ctx: &'b NodeCtx<'a>, tag_name: &'static str) -> Self {
        // pub fn new<B>(ctx: &'a mut NodeCtx<'a>, tag_name: &'a str) -> Self {
        let bump = ctx.bump;
        ElementBuilder {
            ctx,
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
            ctx: self.ctx,
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
            ctx: self.ctx,
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
            ctx: self.ctx,
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
    pub fn namespace(self, namespace: Option<&'a str>) -> Self {
        ElementBuilder {
            ctx: self.ctx,
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
    pub fn key(mut self, key: u32) -> Self {
        use std::u32;
        debug_assert!(key != u32::MAX);
        self.key = NodeKey(key);
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
        let children: &'a Children = self.ctx.bump.alloc(self.children);
        let children: &'a [VNode<'a>] = children.as_ref();

        let listeners: &'a Listeners = self.ctx.bump.alloc(self.listeners);
        let listeners: &'a [Listener<'a>] = listeners.as_ref();

        let attributes: &'a Attributes = self.ctx.bump.alloc(self.attributes);
        let attributes: &'a [Attribute<'a>] = attributes.as_ref();

        VNode::element(
            self.ctx.bump,
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
    #[inline]
    pub fn on(mut self, event: &'static str, callback: impl Fn(VirtualEvent) + 'a) -> Self {
        // todo:
        // increment listner id from nodectx ref
        // add listener attrs here instead of later?
        self.listeners.push(Listener {
            event,
            callback: self.ctx.bump.alloc(callback),
            id: *self.ctx.idx.borrow(),
            scope: self.ctx.scope,
        });

        // bump the context id forward
        *self.ctx.idx.borrow_mut() += 1;
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
    #[inline]
    pub fn attr(mut self, name: &'static str, value: &'a str) -> Self {
        self.attributes.push(Attribute { name, value });
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
            self.attributes.push(Attribute { name, value: "" });
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

    // pub fn virtual_child(mut self)
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
// pub fn text<'a>(contents: &'a str) -> VNode<'a> {
//     VNode::text(contents)
// }

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
    Attribute { name, value }
}

// /// Create an event listener.
// ///
// /// `event` is the type of event to listen for, e.g. `"click"`. The `callback`
// /// is the function that will be invoked when the event occurs.
// ///
// /// # Example
// ///
// /// ```no_run
// /// use dioxus::{builder::*, bumpalo::Bump};
// ///
// /// let b = Bump::new();
// ///
// /// let listener = on(&b, "click", |root, vdom, event| {
// ///     // do something when a click happens...
// /// });
// /// ```
// pub fn on<'a, 'b>(
//     // pub fn on<'a, 'b, F: 'static>(
//     bump: &'a Bump,
//     event: &'static str,
//     callback: impl Fn(VirtualEvent) + 'a,
// ) -> Listener<'a> {
//     Listener {
//         event,
//         callback: bump.alloc(callback),
//     }
// }

pub fn virtual_child<'a, T>(_bump: &'a Bump, _props: T, _f: crate::innerlude::FC<T>) -> VNode<'a> {
    todo!()
    // VNode::Component()
}
