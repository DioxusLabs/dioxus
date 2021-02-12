#![allow(unused)]
//! Example of components in

use std::borrow::Borrow;

use dioxus_core::prelude::*;

fn main() {}

static Header: FC<()> = |ctx, props| {
    let inner = use_ref(&ctx, || 0);

    let handler1 = move || println!("Value is {}", inner.current());

    ctx.view(html! {
        <div>
            <h1> "This is the header bar" </h1>
            <h1> "Idnt it awesome" </h1>
            <button onclick={move |_| handler1()}> "Click me" </button>
        </div>
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

static Example: FC<()> = |ctx, props| {
    ctx.view(html! {
        <div>
            <h1> "BROSKI!" </h1>
            <h1> "DRO!" </h1>
        </div>
    })
};
