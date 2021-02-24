//! Helpers for building virtual DOM VNodes.

use crate::{
    innerlude::VComponent,
    nodes::{Attribute, Listener, NodeKey, VNode},
    prelude::VElement,
};

use bumpalo::Bump;

/// A virtual DOM element builder.
///
/// Typically constructed with element-specific constructors, eg the `div`
/// function for building `<div>` elements or the `button` function for building
/// `<button>` elements.
#[derive(Clone, Debug)]
pub struct ElementBuilder<'a, Listeners, Attributes, Children>
where
    Listeners: 'a + AsRef<[Listener<'a>]>,
    Attributes: 'a + AsRef<[Attribute<'a>]>,
    Children: 'a + AsRef<[VNode<'a>]>,
{
    bump: &'a Bump,
    key: NodeKey,
    tag_name: &'a str,
    listeners: Listeners,
    attributes: Attributes,
    children: Children,
    namespace: Option<&'a str>,
}

impl<'a>
    ElementBuilder<
        'a,
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
    pub fn new<B>(bump: B, tag_name: &'a str) -> Self
    where
        B: Into<&'a Bump>,
    {
        let bump = bump.into();
        ElementBuilder {
            bump,
            key: NodeKey::NONE,
            tag_name,
            listeners: bumpalo::collections::Vec::new_in(bump),
            attributes: bumpalo::collections::Vec::new_in(bump),
            children: bumpalo::collections::Vec::new_in(bump),
            namespace: None,
        }
    }
}

impl<'a, Listeners, Attributes, Children> ElementBuilder<'a, Listeners, Attributes, Children>
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
    pub fn listeners<L>(self, listeners: L) -> ElementBuilder<'a, L, Attributes, Children>
    where
        L: 'a + AsRef<[Listener<'a>]>,
    {
        ElementBuilder {
            bump: self.bump,
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
    pub fn attributes<A>(self, attributes: A) -> ElementBuilder<'a, Listeners, A, Children>
    where
        A: 'a + AsRef<[Attribute<'a>]>,
    {
        ElementBuilder {
            bump: self.bump,
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
    pub fn children<C>(self, children: C) -> ElementBuilder<'a, Listeners, Attributes, C>
    where
        C: 'a + AsRef<[VNode<'a>]>,
    {
        ElementBuilder {
            bump: self.bump,
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
            bump: self.bump,
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
        let children: &'a Children = self.bump.alloc(self.children);
        let children: &'a [VNode<'a>] = children.as_ref();

        let listeners: &'a Listeners = self.bump.alloc(self.listeners);
        let listeners: &'a [Listener<'a>] = listeners.as_ref();

        let attributes: &'a Attributes = self.bump.alloc(self.attributes);
        let attributes: &'a [Attribute<'a>] = attributes.as_ref();

        VNode::element(
            self.bump,
            self.key,
            self.tag_name,
            listeners,
            attributes,
            children,
            self.namespace,
        )
    }
}

impl<'a, Attributes, Children>
    ElementBuilder<'a, bumpalo::collections::Vec<'a, Listener<'a>>, Attributes, Children>
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
    ///     .on("click", |root, vdom, event| {
    ///         // ...
    ///     })
    ///     .finish();
    /// ```
    #[inline]
    pub fn on(self, _event: &'a str, _callback: impl Fn(()) + 'a) -> Self
// pub fn on<F>(mut self, event: &'a str, callback: impl Fn(()) -> () + 'static) -> Self
// F: Fn(()) + 'static,
        // F: Fn(()) + 'a,
    {
        // self.listeners.push(Listener {
        //     event,
        //     callback: self.bump.alloc(callback),
        // });
        self
    }
}

impl<'a, Listeners, Children>
    ElementBuilder<'a, Listeners, bumpalo::collections::Vec<'a, Attribute<'a>>, Children>
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

impl<'a, Listeners, Attributes>
    ElementBuilder<'a, Listeners, Attributes, bumpalo::collections::Vec<'a, VNode<'a>>>
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
}

macro_rules! builder_constructors {
    ( $(
        $(#[$attr:meta])*
        $name:ident;
    )* ) => {
        $(
            $(#[$attr])*
            #[inline]
            pub fn $name<'a, B>(
                bump: B,
            ) -> ElementBuilder<
                'a,
                bumpalo::collections::Vec<'a, Listener<'a>>,
                bumpalo::collections::Vec<'a, Attribute<'a>>,
                bumpalo::collections::Vec<'a, VNode<'a>>,
            >
            where
                B: Into<&'a Bump>
            {
                ElementBuilder::new(bump, stringify!($name))
            }
        )*
    };
    ( $(
        $(#[$attr:meta])*
        $name:ident <> $namespace:tt;
    )* ) => {
        $(
            $(#[$attr])*
            #[inline]
            pub fn $name<'a>(
                bump: &'a Bump,
            ) -> ElementBuilder<
                'a,
                bumpalo::collections::Vec<'a, Listener<'a>>,
                bumpalo::collections::Vec<'a, Attribute<'a>>,
                bumpalo::collections::Vec<'a, VNode<'a>>,
            > {
                let builder = ElementBuilder::new(bump, stringify!($name));
                builder.namespace(Some($namespace))
            }
        )*
    }
}

// Organized in the same order as
// https://developer.mozilla.org/en-US/docs/Web/HTML/Element
//
// Does not include obsolete elements.
builder_constructors! {
    // Document metadata

    /// Build a
    /// [`<base>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/base)
    /// element.
    base;
    /// Build a
    /// [`<head>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/head)
    /// element.
    head;
    /// Build a
    /// [`<link>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/link)
    /// element.
    link;
    /// Build a
    /// [`<meta>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meta)
    /// element.
    meta;
    /// Build a
    /// [`<style>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/style)
    /// element.
    style;
    /// Build a
    /// [`<title>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/title)
    /// element.
    title;

    // Sectioning root

    /// Build a
    /// [`<body>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/body)
    /// element.
    body;

    // Content sectioning

    /// Build a
    /// [`<address>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/address)
    /// element.
    address;
    /// Build a
    /// [`<article>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/article)
    /// element.
    article;
    /// Build a
    /// [`<aside>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/aside)
    /// element.
    aside;
    /// Build a
    /// [`<footer>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/footer)
    /// element.
    footer;
    /// Build a
    /// [`<header>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/header)
    /// element.
    header;
    /// Build a
    /// [`<h1>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h1)
    /// element.
    h1;
    /// Build a
    /// [`<h2>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h2)
    /// element.
    h2;
    /// Build a
    /// [`<h3>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h3)
    /// element.
    h3;
    /// Build a
    /// [`<h4>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h4)
    /// element.
    h4;
    /// Build a
    /// [`<h5>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h5)
    /// element.
    h5;
    /// Build a
    /// [`<h6>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/h6)
    /// element.
    h6;
    /// Build a
    /// [`<hgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hgroup)
    /// element.
    hgroup;
    /// Build a
    /// [`<main>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/main)
    /// element.
    main;
    /// Build a
    /// [`<nav>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/nav)
    /// element.
    nav;
    /// Build a
    /// [`<section>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/section)
    /// element.
    section;

    // Text content

    /// Build a
    /// [`<blockquote>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/blockquote)
    /// element.
    blockquote;
    /// Build a
    /// [`<dd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dd)
    /// element.
    dd;
    /// Build a
    /// [`<div>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div)
    /// element.
    div;
    /// Build a
    /// [`<dl>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dl)
    /// element.
    dl;
    /// Build a
    /// [`<dt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dt)
    /// element.
    dt;
    /// Build a
    /// [`<figcaption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figcaption)
    /// element.
    figcaption;
    /// Build a
    /// [`<figure>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/figure)
    /// element.
    figure;
    /// Build a
    /// [`<hr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/hr)
    /// element.
    hr;
    /// Build a
    /// [`<li>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/li)
    /// element.
    li;
    /// Build a
    /// [`<ol>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ol)
    /// element.
    ol;
    /// Build a
    /// [`<p>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/p)
    /// element.
    p;
    /// Build a
    /// [`<pre>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/pre)
    /// element.
    pre;
    /// Build a
    /// [`<ul>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ul)
    /// element.
    ul;

    // Inline text semantics

    /// Build a
    /// [`<a>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/a)
    /// element.
    a;
    /// Build a
    /// [`<abbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/abbr)
    /// element.
    abbr;
    /// Build a
    /// [`<b>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/b)
    /// element.
    b;
    /// Build a
    /// [`<bdi>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdi)
    /// element.
    bdi;
    /// Build a
    /// [`<bdo>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/bdo)
    /// element.
    bdo;
    /// Build a
    /// [`<br>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/br)
    /// element.
    br;
    /// Build a
    /// [`<cite>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/cite)
    /// element.
    cite;
    /// Build a
    /// [`<code>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/code)
    /// element.
    code;
    /// Build a
    /// [`<data>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/data)
    /// element.
    data;
    /// Build a
    /// [`<dfn>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dfn)
    /// element.
    dfn;
    /// Build a
    /// [`<em>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/em)
    /// element.
    em;
    /// Build a
    /// [`<i>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/i)
    /// element.
    i;
    /// Build a
    /// [`<kbd>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/kbd)
    /// element.
    kbd;
    /// Build a
    /// [`<mark>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/mark)
    /// element.
    mark;
    /// Build a
    /// [`<q>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/q)
    /// element.
    q;
    /// Build a
    /// [`<rb>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rb)
    /// element.
    rb;
    /// Build a
    /// [`<rp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rp)
    /// element.
    rp;
    /// Build a
    /// [`<rt>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rt)
    /// element.
    rt;
    /// Build a
    /// [`<rtc>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/rtc)
    /// element.
    rtc;
    /// Build a
    /// [`<ruby>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ruby)
    /// element.
    ruby;
    /// Build a
    /// [`<s>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/s)
    /// element.
    s;
    /// Build a
    /// [`<samp>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/samp)
    /// element.
    samp;
    /// Build a
    /// [`<small>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/small)
    /// element.
    small;
    /// Build a
    /// [`<span>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/span)
    /// element.
    span;
    /// Build a
    /// [`<strong>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/strong)
    /// element.
    strong;
    /// Build a
    /// [`<sub>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sub)
    /// element.
    sub;
    /// Build a
    /// [`<sup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/sup)
    /// element.
    sup;
    /// Build a
    /// [`<time>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/time)
    /// element.
    time;
    /// Build a
    /// [`<u>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/u)
    /// element.
    u;
    /// Build a
    /// [`<var>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/var)
    /// element.
    var;
    /// Build a
    /// [`<wbr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/wbr)
    /// element.
    wbr;

    // Image and multimedia

    /// Build a
    /// [`<area>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/area)
    /// element.
    area;
    /// Build a
    /// [`<audio>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio)
    /// element.
    audio;
    /// Build a
    /// [`<img>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/img)
    /// element.
    img;
    /// Build a
    /// [`<map>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/map)
    /// element.
    map;
    /// Build a
    /// [`<track>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/track)
    /// element.
    track;
    /// Build a
    /// [`<video>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/video)
    /// element.
    video;

    // Embedded content

    /// Build a
    /// [`<embed>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/embed)
    /// element.
    embed;
    /// Build a
    /// [`<iframe>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/iframe)
    /// element.
    iframe;
    /// Build a
    /// [`<object>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/object)
    /// element.
    object;
    /// Build a
    /// [`<param>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/param)
    /// element.
    param;
    /// Build a
    /// [`<picture>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/picture)
    /// element.
    picture;
    /// Build a
    /// [`<source>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/source)
    /// element.
    source;

    // Scripting

    /// Build a
    /// [`<canvas>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/canvas)
    /// element.
    canvas;
    /// Build a
    /// [`<noscript>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/noscript)
    /// element.
    noscript;
    /// Build a
    /// [`<script>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/script)
    /// element.
    script;

    // Demarcating edits

    /// Build a
    /// [`<del>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/del)
    /// element.
    del;
    /// Build a
    /// [`<ins>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/ins)
    /// element.
    ins;

    // Table content

    /// Build a
    /// [`<caption>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/caption)
    /// element.
    caption;
    /// Build a
    /// [`<col>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/col)
    /// element.
    col;
    /// Build a
    /// [`<colgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/colgroup)
    /// element.
    colgroup;
    /// Build a
    /// [`<table>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/table)
    /// element.
    table;
    /// Build a
    /// [`<tbody>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tbody)
    /// element.
    tbody;
    /// Build a
    /// [`<td>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/td)
    /// element.
    td;
    /// Build a
    /// [`<tfoot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tfoot)
    /// element.
    tfoot;
    /// Build a
    /// [`<th>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/th)
    /// element.
    th;
    /// Build a
    /// [`<thead>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/thead)
    /// element.
    thead;
    /// Build a
    /// [`<tr>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/tr)
    /// element.
    tr;

    // Forms

    /// Build a
    /// [`<button>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/button)
    /// element.
    button;
    /// Build a
    /// [`<datalist>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/datalist)
    /// element.
    datalist;
    /// Build a
    /// [`<fieldset>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/fieldset)
    /// element.
    fieldset;
    /// Build a
    /// [`<form>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/form)
    /// element.
    form;
    /// Build a
    /// [`<input>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input)
    /// element.
    input;
    /// Build a
    /// [`<label>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/label)
    /// element.
    label;
    /// Build a
    /// [`<legend>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/legend)
    /// element.
    legend;
    /// Build a
    /// [`<meter>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/meter)
    /// element.
    meter;
    /// Build a
    /// [`<optgroup>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/optgroup)
    /// element.
    optgroup;
    /// Build a
    /// [`<option>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/option)
    /// element.
    option;
    /// Build a
    /// [`<output>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/output)
    /// element.
    output;
    /// Build a
    /// [`<progress>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/progress)
    /// element.
    progress;
    /// Build a
    /// [`<select>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/select)
    /// element.
    select;
    /// Build a
    /// [`<textarea>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/textarea)
    /// element.
    textarea;

    // Interactive elements

    /// Build a
    /// [`<details>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/details)
    /// element.
    details;
    /// Build a
    /// [`<dialog>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog)
    /// element.
    dialog;
    /// Build a
    /// [`<menu>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/menu)
    /// element.
    menu;
    /// Build a
    /// [`<menuitem>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/menuitem)
    /// element.
    menuitem;
    /// Build a
    /// [`<summary>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/summary)
    /// element.
    summary;

    // Web components

    /// Build a
    /// [`<slot>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/slot)
    /// element.
    slot;
    /// Build a
    /// [`<template>`](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/template)
    /// element.
    template;
}

builder_constructors! {
    // SVG components

    /// Build a
    /// [`<svg>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/svg)
    /// element.
    svg <> "http://www.w3.org/2000/svg" ;
    /// Build a
    /// [`<path>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/path)
    /// element.
    path <> "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<circle>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/circle)
    /// element.
    circle <>  "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<ellipse>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/ellipse)
    /// element.
    ellipse <> "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<line>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/line)
    /// element.
    line <> "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<polygon>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polygon)
    /// element.
    polygon <> "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<polyline>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/polyline)
    /// element.
    polyline <> "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<rect>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/rect)
    /// element.
    rect <> "http://www.w3.org/2000/svg";
    /// Build a
    /// [`<image>`](https://developer.mozilla.org/en-US/docs/Web/SVG/Element/image)
    /// element.
    image <> "http://www.w3.org/2000/svg";
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

/// Create an event listener.
///
/// `event` is the type of event to listen for, e.g. `"click"`. The `callback`
/// is the function that will be invoked when the event occurs.
///
/// # Example
///
/// ```no_run
/// use dioxus::{builder::*, bumpalo::Bump};
///
/// let b = Bump::new();
///
/// let listener = on(&b, "click", |root, vdom, event| {
///     // do something when a click happens...
/// });
/// ```
pub fn on<'a, 'b, F: 'static>(
    bump: &'a Bump,
    event: &'static str,
    callback: impl Fn(()) + 'a,
) -> Listener<'a> {
    Listener {
        event,
        callback: bump.alloc(callback),
    }
}

pub fn virtual_child<'a, T>(_bump: &'a Bump, _props: T, _f: crate::innerlude::FC<T>) -> VNode<'a> {
    todo!()
    // VNode::Component()
}
