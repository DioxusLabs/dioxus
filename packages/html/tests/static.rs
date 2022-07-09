use std::{any::Any, ops::Deref, ptr::addr_of};
//
use dioxus_core::prelude::*;

/*

todo:
- allow lowercase components
- functions can be builders however they want to be
- merge implementations of elements and components
*/

struct NodeBuilder<'a, P> {
    inner: Scope<'a>,
    children: &'a [VNode<'a>],
    parent: P,
}

impl<'a> NodeBuilder<'a, ()> {
    fn class(self, f: &str) -> Self {
        todo!()
    }

    fn onclick(&self, f: impl FnMut(())) -> Self {
        todo!()
    }

    fn children<const NUM: usize>(self, children: [Element<'a>; NUM]) -> Self {
        todo!()
    }

    fn build(self) -> Element<'a> {
        todo!()
    }
}

struct Base<'a> {
    inner: &'a ScopeState,
}

trait MyThing<T> {}

trait Buildable<'a, P, O> {
    type Builder;
}

impl<'a, F, I> Buildable<'a, (), NodeBuilder<'a, I>> for F
where
    F: Fn(&ScopeState) -> NodeBuilder<I>,
{
    type Builder = NodeBuilder<'static, I>;
}

struct Template {}

impl<'a, F> Buildable<'a, (), Template> for F
where
    F: Fn(&'a ScopeState) -> Element<'a>,
{
    type Builder = Element<'a>;
}

impl<'a, F, P> Buildable<'a, P, Element<'static>> for F
where
    F: Fn(Scope<P>) -> Element,
    P: Properties,
{
    type Builder = P::Builder;
}

fn buildit<'a, P: Properties, O, C: Buildable<'a, P, O>>(
    f: C,
    g: *const (),
    name: &'static str,
) -> C::Builder {
    todo!()
}

fn base(cx: &ScopeState) -> NodeBuilder<()> {
    todo!()
}

fn my_component(cx: Scope) -> Element {
    todo!()
}

fn dis_ambiguate(cx: Scope) {
    // Used by builder
    let builder = base(&cx);

    // Used by the macro
    let r = buildit(base, base as _, "base").class("asda").build();
    let r = buildit(my_component, my_component as _, "name").build();
    let r = buildit(mytemplate, my_component as _, "custom_template").build();
}

trait MyBuilIt<'a> {
    fn build(self) -> Element<'a>;
}

impl<'a> MyBuilIt<'a> for Element<'a> {
    fn build(self) -> Element<'a> {
        todo!()
    }
}

// Components:
// - name
// - ptr
// - children
// - manual props

// elements
// - name
// - ptr
// - children
// - attributes

// Purely static builder
fn mytemplate(s: &ScopeState) -> Element {
    base(s)
        .class("asda")
        .class("asda")
        .class("asda")
        .class("asda")
        .class("asda")
        .onclick(move |_| {
            //
        })
        .children([
            base(s).class("asda").build(),
            base(s).class("asda").build(),
            base(s).class("asda").build(),
            base(s).class("asda").build(),
            base(s).class("asda").build(),
            base(s).class("asda").build(),
            buildit(my_component, my_component as _, "name").build(),
        ])
        .build()
}

#[test]
fn static_builder() {
    struct Builder {
        array: &'static [(&'static str, &'static str)],
    }
}
