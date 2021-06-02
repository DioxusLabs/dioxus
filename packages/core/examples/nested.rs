#![allow(unused)]
//! Example of components in

use std::{borrow::Borrow, marker::PhantomData};

use dioxus_core::prelude::*;

fn main() {}

static Header: FC<()> = |ctx| {
    let inner = use_ref(ctx, || 0);

    let handler1 = move || println!("Value is {}", inner.borrow());

    ctx.render(dioxus::prelude::LazyNodes::new(|nodectx| {
        builder::ElementBuilder::new(nodectx, "div")
            .child(VNode::Component(nodectx.bump().alloc(VComponent::new(
                Bottom,
                (),
                None,
            ))))
            .finish()
    }))
};

static Bottom: FC<()> = |ctx| {
    ctx.render(html! {
        <div>
            <h1> "bruh 1" </h1>
            <h1> "bruh 2" </h1>
        </div>
    })
};

fn Top(ctx: Context<()>) -> VNode {
    ctx.render(html! {
        <div>
            <h1> "bruh 1" </h1>
            <h1> "bruh 2" </h1>
        </div>
    })
}
