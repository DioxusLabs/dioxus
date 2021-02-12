#![allow(unused)]
//! Example of components in

use std::borrow::Borrow;

use dioxus_core::prelude::*;

fn main() {}

static Header: FC<()> = |ctx, props| {
    let inner = use_ref(&ctx, || 0);

    let handler1 = move || println!("Value is {}", inner.current());

    ctx.view(|bump| {
        builder::div(bump)
            .child(VNode::Component(VComponent::new(
                Bottom,
                //
                (),
            )))
            .finish()
    })
};

static Bottom: FC<()> = |ctx, props| {
    ctx.view(html! {
        <div>
            <h1> "bruh 1" </h1>
            <h1> "bruh 2" </h1>
        </div>
    })
};
