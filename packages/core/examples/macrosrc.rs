#![allow(unused, non_upper_case_globals)]
use bumpalo::Bump;
use dioxus_core::prelude::*;
use dioxus_core::{nodebuilder::*, virtual_dom::DomTree};
use std::{collections::HashMap, future::Future, marker::PhantomData};

fn main() {}
struct Props<'a> {
    use_macro: bool,

    // todo uh not static
    // incorporate lifetimes into the thing somehow
    text: &'a str,
}

fn Component(ctx: Context<Props>) -> VNode {
    // Write asynchronous rendering code that immediately returns a "suspended" VNode
    // The concurrent API will then progress this component when the future finishes
    // You can suspend the entire component, or just parts of it
    let product_list = ctx.suspend(async {
        // Suspend the rendering that completes when the future is done
        match fetch_data().await {
            Ok(data) => html! {<div> </div>},
            Err(_) => html! {<div> </div>},
        }
    });

    // VNodes can be constructed via a builder or the html! macro
    // However, both of these are "lazy" - they need to be evaluated (aka, "viewed")
    // We can "view" them with Context for ultimate speed while inside components
    if ctx.props.use_macro {
        ctx.view(|bump| {
            div(bump)
                .attr("class", "edit")
                .child(text("Hello"))
                .child(text(ctx.props.text))
                .finish()
        })
    } else {
        // "View" indicates exactly *when* allocations take place, everything is lazy up to this point
        ctx.view(html! {
            <div>
                // TODO!
                // Get all this working again
                // <h1>"Products"</h1>
                // // Subnodes can even be suspended
                // // When completely rendered, they won't cause the component itself to re-render, just their slot
                // <p> {product_list} </p>
            </div>
        })
    }
}

/// An example of a datafetching service
async fn fetch_data() -> Result<String, ()> {
    todo!()
}
