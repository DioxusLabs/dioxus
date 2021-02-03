#![allow(unused, non_upper_case_globals, non_snake_case)]
use bumpalo::Bump;
use dioxus_core::prelude::*;
use dioxus_core::{nodebuilder::*, virtual_dom::DomTree};
use std::{collections::HashMap, future::Future, marker::PhantomData};

fn main() {}

// ~~~ Text shared between components via props can be done with lifetimes! ~~~
// Super duper efficient :)
struct Props {
    blah: bool,
    text: String,
}

fn Component<'a>(ctx: &'a Context<Props>) -> VNode<'a> {
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

    ctx.view(html! {
        <div>
            // <h1> "Products" </h1>
            // // Subnodes can even be suspended
            // // When completely rendered, they won't cause the component itself to re-render, just their slot
            // <p> { product_list } </p>
        </div>
    })
}

fn BuilderComp(ctx: Context<Props>) -> VNode {
    // VNodes can be constructed via a builder or the html! macro
    // However, both of these are "lazy" - they need to be evaluated (aka, "viewed")
    // We can "view" them with Context for ultimate speed while inside components
    ctx.view(|bump| {
        div(bump)
            .attr("class", "edit")
            .child(text("Hello"))
            .child(text(ctx.props.text.as_str()))
            .finish()
    })
}

#[fc]
fn EffcComp(ctx: &Context, name: &str) -> VNode {
    // VNodes can be constructed via a builder or the html! macro
    // However, both of these are "lazy" - they need to be evaluated (aka, "viewed")
    // We can "view" them with Context for ultimate speed while inside components
    // use "phase" style allocation;
    /*
    nodes...
    text...
    attrs...
    <div> // node0
        <div> </div> // node1
        {// support some expression} // node 2
    </div>
    let node0;
    let node1;
    let node2 = evaluate{}.into();
    let g= |bump| {1};
    g(bump).into()

    */

    // should we automatically view the output or leave it?
    ctx.view(html! {
        <div>
            // your template goes here
            // feel free to directly use "name"
        </div>
    })
}

fn FullySuspended(ctx: Context<Props>) -> VNode {
    ctx.suspend(async {
        let i: i32 = 0;

        // full suspended works great with just returning VNodes!
        let tex = match i {
            1 => html! { <div> </div> },
            2 => html! { <div> </div> },
            _ => html! { <div> </div> },
        };

        if ctx.props.blah {
            html! { <div> </div> }
        } else {
            tex
        }
    })
}

/// An example of a datafetching service
async fn fetch_data() -> Result<String, ()> {
    todo!()
}
