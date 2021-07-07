//! Demonstrate that borrowed data is possible as a property type
//! Borrowing (rather than cloning) is very important for speed and ergonomics.
//!
//! It's slightly more advanced than just cloning, but well worth the investment.
//!
//! If you use the FC macro, we handle the lifetimes automatically, making it easy to write efficient & performant components.
fn main() {}

use dioxus_core::prelude::*;
use std::rc::Rc;

struct AppProps {
    items: Vec<Rc<ListItem>>,
}

#[derive(PartialEq)]
struct ListItem {
    name: String,
    age: u32,
}

fn app(cx: Context<AppProps>) -> VNode {
    let (val, set_val) = use_state_classic(cx, || 0);

    cx.render(LazyNodes::new(move |_nodecx| {
        todo!()
        // builder::ElementBuilder::new(_nodecx, "div")
        //     .iter_child({
        //         cx.items.iter().map(|child| {
        //             builder::virtual_child(
        //                 _nodecx,
        //                 ChildItem,
        //                 ChildProps {
        //                     item: child.clone(),
        //                     item_handler: set_val.clone(),
        //                 },
        //                 None,
        //                 &[],
        //             )
        //         })
        //     })
        //     .iter_child([builder::ElementBuilder::new(_nodecx, "div")
        //         .iter_child([builder::text3(_nodecx.bump(), format_args!("{}", val))])
        //         .finish()])
        //     .finish()
    }))
}

// props should derive a partialeq implementation automatically, but implement ptr compare for & fields
struct ChildProps {
    // Pass down complex structs
    item: Rc<ListItem>,

    // Even pass down handlers!
    item_handler: Rc<dyn Fn(i32)>,
}

fn ChildItem<'a>(cx: Context<'a, ChildProps>) -> VNode {
    cx.render(LazyNodes::new(move |__cx| todo!()))
}

impl PartialEq for ChildProps {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
impl Properties for ChildProps {
    type Builder = ();
    fn builder() -> Self::Builder {
        ()
    }
    unsafe fn memoize(&self, other: &Self) -> bool {
        self == other
    }
}
