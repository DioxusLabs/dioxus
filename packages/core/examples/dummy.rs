#![allow(unused, non_upper_case_globals)]
use bumpalo::Bump;
use dioxus_core::prelude::VNode;
use dioxus_core::prelude::*;
use dioxus_core::{nodebuilder::*, virtual_dom::Properties};
use once_cell::sync::{Lazy, OnceCell};
use std::{collections::HashMap, future::Future, marker::PhantomData};

fn main() {}

// struct VC<P, F = fn(Context<P>) -> VNode> {
//     f: F,
//     _a: std::marker::PhantomData<(P, F)>, // cell: OnceCell<T>,
//                                           // init: Cell<Option<F>>
// }
// impl<P, F> VC<P, F> {
//     const fn new(init: F) -> VC<P, F> {
//         Self {
//             _a: std::marker::PhantomData {},
//             f: init,
//         }
//     }
//     fn builder() -> P {
//         // P::new()
//     }
// }

// // Build a new functional component
// static SomeComp: VC<()> = VC::new(|ctx| {
//     // This is a component, apparently
//     // still not useful because we can't have bounds

//     ctx.view(html! {
//         <div>

//         </div>
//     })
// });

/*











*/
static BILL: Lazy<fn(Context<()>) -> String> = Lazy::new(|| {
    //
    |c| "BLAH".to_string()
});

// struct FUNC<F = fn() -> T> {}

struct SomeBuilder {}

// struct DummyRenderer {
//     alloc: Bump,
// }

// impl DummyRenderer {
//     // "Renders" a domtree by logging its children and outputs
//     fn render() {}

//     // Takes a domtree, an initial value, a new value, and produces the diff list
//     fn produce_diffs() {}
// }

// struct Props<'a> {
//     name: &'a str,
// }

// /// This component does "xyz things"
// /// This is sample documentation
// static Component: FC<Props> = |ctx| {
//     // This block mimics that output of the html! macro

//     DomTree::new(move |bump| {
//         // parse into RSX structures
//         // regurgetate as rust types

//         // <div> "Child 1" "Child 2"</div>
//         div(bump)
//             .attr("class", "edit")
//             .child(text("Child 1"))
//             .child(text("Child 2"))
//             .finish()
//     })
// };

// /*
// source
//     |> c1 -> VNode
//     |> c2 -> VNode
//     |> c3 -> VNode
//     |> c4 -> VNode
// */
