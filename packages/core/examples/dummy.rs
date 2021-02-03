// #![allow(unused, non_upper_case_globals)]
// use bumpalo::Bump;
// use dioxus_core::nodebuilder::*;
// use dioxus_core::{nodes::DomTree, prelude::*};
// use std::{collections::HashMap, future::Future, marker::PhantomData};

fn main() {}

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
