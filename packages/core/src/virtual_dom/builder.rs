#![allow(missing_docs)]
use bumpalo::Bump;

use crate::{
    innerlude::{empty_cell, BBox, BString, BVec, Component},
    AnyEvent, Attribute, AttributeValue, IntoVNode, LazyNodes, Listener, NodeFactory, UiEvent,
    VElement, VNode, VText,
};

pub fn dom<'a, 'b, T>(f: impl FnOnce(DomBuilder<'a>) -> T + 'a + 'b) -> LazyNodes<'a, 'b>
where
    T: IntoVNode<'a>,
{
    LazyNodes::new(|factory| f(DomBuilder { factory }).into_vnode(factory))
}

#[derive(Clone, Copy)]
pub struct DomBuilder<'a> {
    factory: NodeFactory<'a>,
}

impl<'a> DomBuilder<'a> {
    #[inline]
    pub fn element(&self, tag: &'static str) -> ElementBuilder<'a> {
        ElementBuilder::new(*self, tag)
    }

    #[inline]
    pub fn component<C>(&self, component: C) -> ComponentBuilder<'a, C>
    where
        C: Component,
        C::Props: Default + 'a,
    {
        ComponentBuilder::new(*self, component, Default::default())
    }

    #[inline]
    pub fn component_with<C>(&self, component: C, props: C::Props) -> ComponentBuilder<'a, C>
    where
        C: Component,
        C::Props: 'a,
    {
        ComponentBuilder::new(*self, component, props)
    }

    #[inline]
    pub fn text(&self, text: &str) -> VNode<'a> {
        VNode::Text(self.bump().alloc(VText {
            id: empty_cell(),
            text: BString::from_str_in(text, self.factory.bump()).into_bump_str(),
            is_static: false,
        }))
    }

    #[inline]
    pub fn text_static(&self, text: &'static str) -> VNode<'a> {
        VNode::Text(self.bump().alloc(VText {
            id: empty_cell(),
            text,
            is_static: false,
        }))
    }

    #[inline]
    fn bump(&self) -> &'a Bump {
        self.factory.bump()
    }
}

pub struct ComponentBuilder<'a, C>
where
    C: Component,
{
    builder: DomBuilder<'a>,
    props: C::Props,
    component: C,
    key: Option<&'a str>,
}

impl<'a, C> ComponentBuilder<'a, C>
where
    C: Component,
    C::Props: 'a,
{
    #[inline]
    fn new(builder: DomBuilder<'a>, component: C, props: C::Props) -> Self {
        Self {
            builder,
            component,
            props,
            key: None,
        }
    }

    #[inline]
    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(BString::from_str_in(key, self.builder.factory.bump()).into_bump_str());
        self
    }

    #[inline]
    pub fn key_static(mut self, key: &'static str) -> Self {
        self.key = Some(key);
        self
    }

    pub fn build(self) -> VNode<'a> {
        self.into()
    }
}

impl<'a, C: Component> From<ComponentBuilder<'a, C>> for VNode<'a>
where
    C::Props: 'a,
{
    fn from(c: ComponentBuilder<'a, C>) -> Self {
        c.builder
            .factory
            .component(c.component.renderer(), c.props, None, c.component.name())
    }
}

pub struct ElementBuilder<'a> {
    builder: DomBuilder<'a>,
    namespace: Option<&'static str>,
    listeners: BVec<'a, Listener<'a>>,
    attributes: BVec<'a, Attribute<'a>>,
    children: BVec<'a, VNode<'a>>,
    key: Option<&'a str>,
    tag: &'static str,
}

impl<'a> ElementBuilder<'a> {
    #[inline]
    fn new(builder: DomBuilder<'a>, tag: &'static str) -> Self {
        Self {
            builder,
            namespace: None,
            listeners: BVec::new_in(builder.bump()),
            attributes: BVec::new_in(builder.bump()),
            children: BVec::new_in(builder.bump()),
            key: None,
            tag,
        }
    }

    #[inline]
    pub fn namespace(mut self, ns: &'static str) -> Self {
        self.namespace = Some(ns);
        self
    }

    #[inline]
    pub fn attr(mut self, name: &'static str, value: &str) -> Self {
        self.attributes.push(Attribute {
            name,
            value: AttributeValue::Text(
                BString::from_str_in(value, self.builder.bump()).into_bump_str(),
            ),
            is_static: true,
            is_volatile: false,
            namespace: None,
        });
        self
    }

    #[inline]
    pub fn children<I, T>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<VNode<'a>>,
    {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    #[inline]
    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(BString::from_str_in(key, self.builder.factory.bump()).into_bump_str());
        self
    }

    #[inline]
    pub fn key_static(mut self, key: &'static str) -> Self {
        self.key = Some(key);
        self
    }

    #[inline]
    pub fn on<E, F>(self, event: &'static str, callback: F) -> Self
    where
        F: Fn(UiEvent<E>) + 'a,
        E: Send + Sync + 'static,
    {
        self.on_any::<E, _>(event, move |evt: AnyEvent| {
            let event = evt.downcast::<E>().unwrap();
            callback(event)
        })
    }

    #[inline]
    pub fn on_any<E, F>(mut self, event: &'static str, callback: F) -> Self
    where
        F: Fn(AnyEvent) + 'a,
    {
        let bump = self.builder.bump();

        // we can't allocate unsized in bumpalo's box, so we need to craft the box manually
        // safety: this is essentially the same as calling Box::new() but manually
        // The box is attached to the lifetime of the bumpalo allocator
        let cb: &mut dyn FnMut(AnyEvent) = bump.alloc(callback);

        let callback: BBox<dyn FnMut(AnyEvent) + 'a> = unsafe { BBox::from_raw(cb) };

        let handler = bump.alloc(std::cell::RefCell::new(Some(callback)));

        self.listeners
            .push(self.builder.factory.listener(event, handler));
        self
    }

    #[inline]
    pub fn build(self) -> VNode<'a> {
        self.into()
    }
}

impl<'a> From<ElementBuilder<'a>> for VNode<'a> {
    fn from(el: ElementBuilder<'a>) -> Self {
        let mut items = el.builder.factory.scope.items.borrow_mut();
        let listeners = el.listeners.into_bump_slice();

        for listener in listeners {
            let long_listener = unsafe { std::mem::transmute(listener) };
            items.listeners.push(long_listener);
        }

        VNode::Element(el.builder.bump().alloc(VElement {
            tag: el.tag,
            key: el.key,
            namespace: el.namespace,
            listeners,
            attributes: el.attributes.into_bump_slice(),
            children: el.children.into_bump_slice(),
            id: empty_cell(),
            parent: empty_cell(),
        }))
    }
}

impl<'a> IntoVNode<'a> for ElementBuilder<'a> {
    fn into_vnode(self, _cx: NodeFactory<'a>) -> VNode<'a> {
        self.into()
    }
}

// This *could* be more generic, but the main purpose of this
// blanket impl is to allow conveniently turning arrays into fragments, including
// empty ones.
//
// If this was more generic, people would need type annotations for empty arrays.
impl<'a, const N: usize> IntoVNode<'a> for [VNode<'a>; N] {
    fn into_vnode(self, cx: NodeFactory<'a>) -> VNode<'a> {
        cx.fragment_from_iter(IntoIterator::into_iter(self))
    }
}
