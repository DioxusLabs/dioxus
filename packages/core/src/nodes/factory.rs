use crate::{innerlude::*, Attribute, Listener, VElement, VNode, VText};
use bumpalo::{boxed::Box as BumpBox, Bump};
use std::{
    cell::{Cell, RefCell},
    fmt::{Arguments, Debug},
};

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
            id: Default::default(),
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
            id: Default::default(),
        }))
    }

    /// Create a new [`VNode::Element`] without the trait bound
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
            id: Default::default(),
            parent: Default::default(),
        }))
    }

    /// Create a new [`Attribute`]
    pub fn attr(
        &self,
        name: &'static str,
        val: impl IntoAttributeValue<'a>,
        namespace: Option<&'static str>,
        is_volatile: bool,
    ) -> Attribute<'a> {
        Attribute {
            name,
            namespace,
            volatile: is_volatile,
            value: val.into_value(self.bump),
        }
    }

    /// Create a new [`Attribute`] using non-arguments
    pub fn custom_attr(
        &self,
        name: &'static str,
        value: AttributeValue<'a>,
        namespace: Option<&'static str>,
        is_volatile: bool,
    ) -> Attribute<'a> {
        Attribute {
            name,
            namespace,
            volatile: is_volatile,
            value,
        }
    }

    /// Create a new [`VNode::Component`]
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

    /// Create a new [`VNode::Fragment`] from a root of the rsx! call
    pub fn fragment_root<'b, 'c>(
        self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(self.bump);

        for node in node_iter {
            nodes.push(node.into_vnode(self));
        }

        VNode::Fragment(self.bump.alloc(VFragment {
            children: nodes.into_bump_slice(),
            placeholder: Default::default(),
            key: None,
        }))
    }

    /// Create a new [`VNode::Fragment`] from any iterator
    pub fn fragment_from_iter<'c, I, J>(
        self,
        node_iter: impl IntoVNode<'a, I, J> + 'c,
    ) -> VNode<'a> {
        node_iter.into_vnode(self)
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

        let children = nodes.into_bump_slice();

        Some(VNode::Fragment(self.bump.alloc(VFragment {
            children,
            key: None,
            placeholder: Default::default(),
        })))
    }

    /// Create a new [`EventHandler`] from an [`FnMut`]
    pub fn event_handler<T>(self, f: impl FnMut(T) + 'a) -> EventHandler<'a, T> {
        let handler: &mut dyn FnMut(T) = self.bump.alloc(f);
        let caller = unsafe { BumpBox::from_raw(handler as *mut dyn FnMut(T)) };
        let callback = RefCell::new(Some(caller));
        EventHandler { callback }
    }

    /// Create a refrence to a template
    pub fn template_ref(
        &self,
        template: Template,
        nodes: &'a [VNode<'a>],
        attributes: &'a [Attribute<'a>],
        listeners: &'a [Listener<'a>],
        key: Option<Arguments>,
    ) -> VNode<'a> {
        // let borrow_ref = self.scope.templates.borrow();
        // // We only create the template if it doesn't already exist to allow for hot reloading
        // if !borrow_ref.contains_key(&id) {
        //     drop(borrow_ref);
        //     let mut borrow_mut = self.scope.templates.borrow_mut();
        //     borrow_mut.insert(id.clone(), Rc::new(RefCell::new(template)));
        // }
        todo!()
        // VNode::TemplateRef(self.bump.alloc(VTemplate {
        //     dynamic_context,
        //     template_id: id,
        //     node_ids: RefCell::new(Vec::new()),
        //     parent: Cell::new(None),
        //     template_ref_id: Cell::new(None),
        // }))
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
pub trait IntoVNode<'a, I = (), J = ()> {
    /// Convert this into a [`VNode`], using the [`NodeFactory`] as a source of allocation
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a>;
}

// TODO: do we even need this? It almost seems better not to
// // For the case where a rendered VNode is passed into the rsx! macro through curly braces
impl<'a> IntoVNode<'a> for VNode<'a> {
    fn into_vnode(self, _: NodeFactory<'a>) -> VNode<'a> {
        self
    }
}

impl<'a, 'b> IntoVNode<'a> for LazyNodes<'a, '_> {
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

impl<'a> IntoVNode<'a> for &VNode<'a> {
    fn into_vnode(self, _cx: NodeFactory<'a>) -> VNode<'a> {
        // borrowed nodes are strange
        self.decouple()
    }
}

// Note that we're using the E as a generic but this is never crafted anyways.
pub struct FromNodeIterator;
impl<'a, T, I, E> IntoVNode<'a, FromNodeIterator, E> for T
where
    T: IntoIterator<Item = I>,
    I: IntoVNode<'a, E>,
{
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        let mut nodes = bumpalo::collections::Vec::new_in(cx.bump);

        for node in self {
            nodes.push(node.into_vnode(cx));
        }

        let children = nodes.into_bump_slice();

        if cfg!(debug_assertions) && children.len() > 1 && children.last().unwrap().key().is_none()
        {
            // let bt = backtrace::Backtrace::new();
            let bt = "no backtrace available";

            // todo: make the backtrace prettier or remove it altogether
            log::error!(
                r#"
                Warning: Each child in an array or iterator should have a unique "key" prop.
                Not providing a key will lead to poor performance with lists.
                See docs.rs/dioxus for more information.
                -------------
                {:?}
                "#,
                bt
            );
        }

        VNode::Fragment(cx.bump.alloc(VFragment {
            children,
            placeholder: Default::default(),
            key: None,
        }))
    }
}
