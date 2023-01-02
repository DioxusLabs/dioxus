//! Example: README.md showcase
//!
//! The example from the README.md.

use std::{any::Any, cell::RefCell, marker::PhantomData, pin::Pin};

use dioxus::prelude::*;
use futures_util::{future::abortable, Future};

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let name = cx.use_hook(|| "asdasd".to_string());

    cx.spawn_local(async {
        println!("Hello, world! {name}");
    });

    todo!()
}
