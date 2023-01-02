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

    cx.spawn_local(async move {
        println!("Hello, world! From the top-level future {name}");
    });

    cx.render(rsx! {
        div {
            // Child {
            //     onclick: |s| {
            //         println!("Clicked....: {}", s);
            //         cx.spawn_local(async move {
            //             println!("Clicked: {}", s);
            //         });
            //     }
            // }
        }
    })
}

// #[inline_props]

fn Child<'a, 'b>(cx: Scoped<'a, 'b, ()>, onclick: &'a EventHandler<'a, &'a String>) -> Element<'a> {
    let name = cx.use_hook(|| "asdasd".to_string());

    onclick.call(&name);

    spawn_local(cx, async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            println!("Hello, world! from the bottom level future {name}");
        }
    });

    cx.scope.render(rsx! {
        div {
            "Hello, world!"
            button {
                onclick: move |_| {
                    onclick.call(&name);
                },
                "Click to spawn future"
            }
        }
    })
}

pub fn spawn_local<'a, 'b>(cx: Scoped<'a, 'b, ()>, fut: impl Future<Output = ()> + 'b) {
    // self.tasks.spawn_local(self.id, fut);
}
