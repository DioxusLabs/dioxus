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
    pub fn finish(mut self) -> VNode<'a> {
        let bump = self.cx.bump();

        let children: &'a Children = bump.alloc(self.children);
        let children: &'a [VNode<'a>] = children.as_ref();

        let listeners: &'a Listeners = bump.alloc(self.listeners);
        let listeners: &'a [Listener<'a>] = listeners.as_ref();

        for listener in listeners {
            // bump the context id forward
            let id = self.cx.listener_id.get();
            self.cx.listener_id.set(id + 1);

            // Add this listener to the context list
            // This casts the listener to a self-referential pointer
            // This is okay because the bump arena is stable

            // TODO: maybe not add it into CTX immediately
            let r = unsafe { std::mem::transmute::<&Listener<'a>, &Listener<'static>>(listener) };
            self.cx.scope_ref.listeners.borrow_mut().push((
                r.mounted_node as *const _ as *mut _,
                r.callback as *const _ as *mut _,
            ));
        }

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
    // / Add a new event listener to this element.
    // /
    // / The `event` string specifies which event will be listened for. The
    // / `callback` function is the function that will be invoked if the
    // / specified event occurs.
    // /
    // / # Example
    // /
    // / ```no_run
    // / use dioxus::{builder::*, bumpalo::Bump};
    // /
    // / let b = Bump::new();
    // /
    // / // A button that does something when clicked!
    // / let my_button = button(&b)
    // /     .on("click", |event| {
    // /         // ...
    // /     })
    // /     .finish();
    // / ```
    // pub fn on(self, event: &'static str, callback: impl FnMut(VirtualEvent) + 'a) -> Self {
    //     let bump = &self.cx.bump();
    //     let listener = Listener {
    //         event,
    //         callback: bump.alloc(callback),
    //         scope: self.cx.scope_ref.arena_idx,
    //         mounted_node: bump.alloc(Cell::new(RealDomNode::empty())),
    //     };
    //     self.add_listener(listener)
    // }

    // pub fn add_listener(mut self, listener: Listener<'a>) -> Self {
    //     self.listeners.push(listener);

    //     // bump the context id forward
    //     let id = self.cx.listener_id.get();
    //     self.cx.listener_id.set(id + 1);

    //     // Add this listener to the context list
    //     // This casts the listener to a self-referential pointer
    //     // This is okay because the bump arena is stable
    //     self.listeners.last().map(|g| {
    //         let r = unsafe { std::mem::transmute::<&Listener<'a>, &Listener<'static>>(g) };
    //         self.cx.scope_ref.listeners.borrow_mut().push((
    //             r.mounted_node as *const _ as *mut _,
    //             r.callback as *const _ as *mut _,
    //         ));
    //     });

    //     self
    // }
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
        let (value, is_static) = raw_text(self.cx.bump(), args);

        self.attributes.push(Attribute {
            name,
            value,
            is_static,
            namespace: None,
        });
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
        todo!();
        let len_before = self.children.len();
        for item in nodes {
            todo!()
            // let child = item.into_vnode(&self.cx);
            // self.children.push(child);
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
