pub use crate::builder::IntoAttributeValue;
use bumpalo::boxed::Box as BumpBox;
use bumpalo::collections::Vec as BumpVec;
use dioxus_core::{
    self, exports::bumpalo, Attribute, IntoVNode, Listener, NodeFactory, ScopeState, UiEvent,
    VNode, VText,
};

pub struct ElementBuilder<'a> {
    name: &'static str,
    fac: NodeFactory<'a>,
    attrs: BumpVec<'a, Attribute<'a>>,
    children: BumpVec<'a, VNode<'a>>,
    listeners: BumpVec<'a, Listener<'a>>,
    namespace: Option<&'static str>,
    key: Option<&'a str>,
}

impl<'a> IntoVNode<'a> for ElementBuilder<'a> {
    fn into_vnode(self, _cx: NodeFactory<'a>) -> VNode<'a> {
        self.build().unwrap()
    }
}

impl<'a> ElementBuilder<'a> {
    pub fn new(cx: &'a ScopeState, name: &'static str) -> Self {
        let fac = NodeFactory::new(cx);
        ElementBuilder {
            attrs: BumpVec::new_in(fac.bump()),
            children: BumpVec::new_in(fac.bump()),
            listeners: BumpVec::new_in(fac.bump()),
            name,
            fac,
            namespace: None,
            key: None,
        }
    }

    pub fn new_svg(cx: &'a ScopeState, name: &'static str) -> Self {
        let fac = NodeFactory::new(cx);
        ElementBuilder {
            attrs: BumpVec::new_in(fac.bump()),
            children: BumpVec::new_in(fac.bump()),
            listeners: BumpVec::new_in(fac.bump()),
            name,
            fac,
            namespace: Some("http://www.w3.org/2000/svg"),
            key: None,
        }
    }

    pub fn push_attr(&mut self, name: &'static str, val: impl IntoAttributeValue<'a>) {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: None,
            is_volatile: false,
        });
    }

    pub fn push_attr_volatile(&mut self, name: &'static str, val: impl IntoAttributeValue<'a>) {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: None,
            is_volatile: true,
        });
    }

    pub fn push_listener<D: UiEvent>(
        &mut self,
        event_name: &'static str,
        mut callback: impl FnMut(&D) + 'a,
    ) -> Listener<'a> {
        use dioxus_core::AnyEvent;
        let fac = self.fac;
        let bump = fac.bump();

        // we can't allocate unsized in bumpalo's box, so we need to craft the box manually
        // safety: this is essentially the same as calling Box::new() but manually
        // The box is attached to the lifetime of the bumpalo allocator
        let cb: &mut dyn FnMut(AnyEvent) = bump.alloc(move |evt: AnyEvent| {
            let event = evt.downcast::<D>().unwrap();
            callback(event)
        });

        let callback: BumpBox<dyn FnMut(AnyEvent) + 'a> = unsafe { BumpBox::from_raw(cb) };

        // ie copy
        let shortname: &'static str = &event_name[2..];

        let handler = bump.alloc(std::cell::RefCell::new(Some(callback)));
        fac.listener(shortname, handler)
    }

    pub fn key(mut self, key: impl IntoAttributeValue<'a>) -> Self {
        let (value, _) = key.into_str(self.fac);
        self.key = Some(value);
        self
    }

    /// Build this Element builder into a VNode.
    pub fn build(self) -> Option<VNode<'a>> {
        Some(self.fac.raw_element(
            self.name,
            self.namespace,
            self.listeners.into_bump_slice(),
            self.attrs.into_bump_slice(),
            self.children.into_bump_slice(),
            self.key,
        ))
    }

    pub fn hints(mut self, listeners: usize, attrs: usize, children: usize) -> Self {
        self.listeners.reserve(listeners);
        self.attrs.reserve(attrs);
        self.children.reserve(children);
        self
    }

    pub fn attr(mut self, name: &'static str, val: impl IntoAttributeValue<'a>) -> Self {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: None,
            is_volatile: false,
        });
        self
    }

    pub fn style_attr(mut self, name: &'static str, val: impl IntoAttributeValue<'a>) -> Self {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: Some("style"),
            is_volatile: false,
        });
        self
    }

    /// Add a bunch of pre-formatted attributes
    pub fn attributes(mut self, iter: impl IntoIterator<Item = Attribute<'a>>) -> Self {
        for attr in iter {
            self.attrs.push(attr);
        }
        self
    }

    pub fn attr_ns(
        mut self,
        name: &'static str,
        name_space: &'static str,
        val: impl IntoAttributeValue<'a>,
    ) -> Self {
        let (value, is_static) = val.into_str(self.fac);
        self.attrs.push(Attribute {
            name,
            value,
            is_static,
            namespace: Some(name_space),
            is_volatile: false,
        });
        self
    }

    pub fn child(mut self, node: impl IntoVNode<'a>) -> Self {
        self.children.push(node.into_vnode(self.fac));
        self
    }

    pub fn children<'b, 'c, F, A>(mut self, nodes: A) -> Self
    where
        F: IntoVNode<'a>,

        // two trait requirements but we use one
        // this forces all pure iterators to come in as fragments
        A: AsRef<[F]> + IntoIterator<Item = F>,
    {
        for node in nodes {
            self.children.push(node.into_vnode(self.fac));
        }
        self
    }

    /// Add a child fragment from an iterator
    pub fn fragment<'b, 'c>(
        mut self,
        node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
    ) -> Self {
        self.children.push(self.fac.fragment_from_iter(node_iter));
        self
    }
}

pub fn text<'a>(cx: &'a ScopeState, val: impl IntoAttributeValue<'a>) -> VNode<'a> {
    let fac = NodeFactory::new(cx);
    let (text, is_static) = val.into_str(fac);
    VNode::Text(fac.bump().alloc(VText {
        text,
        is_static,
        id: Default::default(),
    }))
}

pub fn fragment<'a, 'b, 'c>(
    cx: &'a ScopeState,
    node_iter: impl IntoIterator<Item = impl IntoVNode<'a> + 'c> + 'b,
) -> VNode<'a> {
    let fac = NodeFactory::new(cx);
    fac.fragment_from_iter(node_iter)
}
