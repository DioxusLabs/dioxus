#![allow(missing_docs)]
use crate::{Element, LazyNodes, Properties, Scope};
use std::any::type_name;

pub trait Component {
    type Props: Properties;

    fn name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn renderer(&self) -> fn(scope: Scope<Self::Props>) -> Element;

    #[inline]
    fn lazy(&self, props: Self::Props) -> LazyNodes {
        LazyNodes::new(move |h| h.component(self.renderer(), props, None, self.name()))
    }
}

pub struct Slot<P: Properties> {
    name: &'static str,
    renderer: fn(scope: Scope<P>) -> Element,
}

impl<P: Properties> Clone for Slot<P> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            renderer: self.renderer,
        }
    }
}

impl<P: Properties> Copy for Slot<P> {}

impl<P: Properties> Component for Slot<P> {
    type Props = P;

    fn renderer(&self) -> fn(scope: Scope<Self::Props>) -> Element {
        self.renderer
    }
}

impl<P: Properties> PartialEq for Slot<P> {
    fn eq(&self, other: &Self) -> bool {
        // FIXME: function equality is not that well-defined,
        // should we just ignore it and compare the name only?
        self.name == other.name && (self.renderer as usize) == (other.renderer as usize)
    }
}

impl<P: Properties> Slot<P> {
    pub fn from_component<C: Component<Props = P>>(component: C) -> Self {
        Self {
            name: component.name(),
            renderer: component.renderer(),
        }
    }

    pub fn from_renderer(name: &'static str, renderer: fn(Scope<P>) -> Element) -> Self {
        Self { name, renderer }
    }

    pub fn empty() -> Self {
        fn _f<P2>(_cx: Scope<P2>) -> Element {
            None
        }

        Self {
            name: "empty slot",
            renderer: _f::<P>,
        }
    }
}
