//! Demonstrate that borrowed data is possible as a property type
//! Borrowing (rather than cloning) is very important for speed and ergonomics.
//!
//! It's slightly more advanced than just cloning, but well worth the investment.
//!
//! If you use the FC macro, we handle the lifetimes automatically, making it easy to write efficient & performant components.

fn main() {}

use std::{borrow::Borrow, ops::Deref, rc::Rc};

use dioxus_core::prelude::*;

struct Props {
    items: Vec<Rc<ListItem>>,
}

#[derive(PartialEq)]
struct ListItem {
    name: String,
    age: u32,
}

fn app(ctx: Context<Props>) -> VNode {
    let (val, set_val) = use_state(&ctx, || 0);

    ctx.render(dioxus::prelude::LazyNodes::new(move |c| {
        let mut root = builder::ElementBuilder::new(c, "div");
        for child in &ctx.items {
            // notice that the child directly borrows from our vec
            // this makes lists very fast (simply views reusing lifetimes)
            // <ChildItem item=child hanldler=setter />
            root = root.child(builder::virtual_child(
                c,
                ChildItem,
                // create the props with nothing but the fc<T>
                fc_to_builder(ChildItem)
                    .item(child.clone())
                    .item_handler(Callback(set_val.clone()))
                    .build(),
                None,
            ));
        }
        root.finish()
    }))
}

// props should derive a partialeq implementation automatically, but implement ptr compare for & fields
#[derive(Props, PartialEq)]
struct ChildProps {
    // Pass down complex structs
    item: Rc<ListItem>,

    // Even pass down handlers!
    item_handler: Callback<i32>,
}

fn ChildItem<'a>(ctx: Context<ChildProps>) -> VNode {
    ctx.render(rsx! {
        div {
            // onclick: move |evt| (ctx.item_handler)(10)
            h1 { "abcd123 {ctx.item.name}" }
            h2 { "abcd123" }
            div {
                "abcd123"
                h2 { }
                p { }
            }
        }
    })
}

#[derive(Clone)]
struct Callback<I, O = ()>(Rc<dyn Fn(I) -> O>);
impl<I, O> Deref for Callback<I, O> {
    type Target = Rc<dyn Fn(I) -> O>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<I, O> PartialEq for Callback<I, O> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
