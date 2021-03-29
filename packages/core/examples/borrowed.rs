//! Demonstrate that borrowed data is possible as a property type
//! Borrowing (rather than cloning) is very important for speed and ergonomics.
//!
//! It's slightly more advanced than just cloning, but well worth the investment.
//!
//! If you use the FC macro, we handle the lifetimes automatically, making it easy to write efficient & performant components.

fn main() {}

use std::borrow::Borrow;

use dioxus_core::prelude::*;

struct Props {
    items: Vec<ListItem>,
}

#[derive(PartialEq)]
struct ListItem {
    name: String,
    age: u32,
}

fn app<'a>(ctx: Context<'a>, props: &Props) -> DomTree {
    let (val, set_val) = use_state(&ctx, || 0);

    ctx.render(dioxus::prelude::LazyNodes::new(move |c| {
        let mut root = builder::ElementBuilder::new(c, "div");
        for child in &props.items {
            // notice that the child directly borrows from our vec
            // this makes lists very fast (simply views reusing lifetimes)
            // <ChildItem item=child hanldler=setter />
            root = root.child(builder::virtual_child(
                c,
                ChildItem,
                // create the props with nothing but the fc<T>
                fc_to_builder(ChildItem)
                    .item(child)
                    .item_handler(set_val)
                    .build(),
            ));
        }
        root.finish()
    }))
}

// props should derive a partialeq implementation automatically, but implement ptr compare for & fields
#[derive(Props)]
struct ChildProps<'a> {
    // Pass down complex structs
    item: &'a ListItem,

    // Even pass down handlers!
    item_handler: &'a dyn Fn(i32),
}

impl PartialEq for ChildProps<'_> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

fn ChildItem<'a>(ctx: Context<'a>, props: &ChildProps) -> DomTree {
    ctx.render(rsx! {
        div {
            onclick: move |evt| (props.item_handler)(10)
            h1 { "abcd123" }
            h2 { "abcd123" }
            div {
                "abcd123"
                h2 { }
                p { }
            }
        }
    })
}
