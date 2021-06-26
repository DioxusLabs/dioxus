// #![allow(unused, non_upper_case_globals, non_snake_case)]
// use bumpalo::Bump;
// use dioxus_core::nodebuilder::*;
// use dioxus_core::prelude::*;
// use std::{collections::HashMap, future::Future, marker::PhantomData};

fn main() {}
// fn main() {
//     let mut vdom = VirtualDom::new_with_props(
//         component,
//         Props {
//             blah: false,
//             text: "blah".into(),
//         },
//     );

//     vdom.progress();

//     let somet = String::from("asd");
//     let text = somet.as_str();

//     /*
//     this could be auto-generated via the macro
//     this props is allocated in this
//     but the component and props would like need to be cached
//     we could box this fn, abstracting away the props requirement and just keep the entrance and allocator requirement
//     How do we keep cached things around?
//     Need some sort of caching mechanism

//     how do we enter into a childscope from a parent scope?

//     Problems:
//     1: Comp props need to be stored somewhere so we can re-evalute components when they receive updates
//     2: Trees are not evaluated

//     */
//     let example_caller = move |cx: &Bump| {
//         todo!()
//         // let p = Props { blah: true, text };
//         // let c = Context { props: &p };
//         // let r = component(&c);
//     };

//     // check the edit list
// }

// // ~~~ Text shared between components via props can be done with lifetimes! ~~~
// // Super duper efficient :)
// struct Props {
//     blah: bool,
//     text: String,
//     // text: &'src str,
// }
// impl Properties for Props {
//     fn new() -> Self {
//         todo!()
//     }
// }

// fn component<'a>(cx: Context<'a, Props>) -> VNode<'a> {
//     // Write asynchronous rendering code that immediately returns a "suspended" VNode
//     // The concurrent API will then progress this component when the future finishes
//     // You can suspend the entire component, or just parts of it
//     let product_list = cx.suspend(async {
//         // Suspend the rendering that completes when the future is done
//         match fetch_data().await {
//             Ok(data) => html! { <div> "success!" </div>},
//             Err(_) => html! { <div> "failure :(" </div>},
//         }
//     });

//     // todo!()
//     cx.render(html! {
//         <div>
//             <h1> "Products" </h1>
//             <button> "hello!" </button>
//             // Subnodes can even be suspended
//             // When completely rendered, they won't cause the component itself to re-render, just their slot

//             // <p> { product_list } </p>
//         </div>
//     })
// }

// fn BuilderComp<'a>(cx: Context<'a, Props>) -> VNode<'a> {
//     // VNodes can be constructed via a builder or the html! macro
//     // However, both of these are "lazy" - they need to be evaluated (aka, "viewed")
//     // We can "view" them with Context for ultimate speed while inside components
//     cx.render(|bump| {
//         div(bump)
//             .attr("class", "edit")
//             .child(text("Hello"))
//             .child(text(cx.cx.text.as_ref()))
//             .finish()
//     })
// }

// #[fc]
// fn EffcComp(cx: Context, name: &str) -> VNode {
//     // VNodes can be constructed via a builder or the html! macro
//     // However, both of these are "lazy" - they need to be evaluated (aka, "viewed")
//     // We can "view" them with Context for ultimate speed while inside components
//     // use "phase" style allocation;

//     cx.render(html! {
//         <div>
//             // your template goes here
//             // feel free to directly use "name"
//         </div>
//     })
// }

// fn FullySuspended<'a>(cx: &'a Context<Props>) -> VNode<'a> {
//     cx.suspend(async {
//         let i: i32 = 0;

//         // full suspended works great with just returning VNodes!
//         // Feel free to capture the html! macro directly
//         // Anything returned here is automatically viewed
//         let tex = match i {
//             1 => html! { <div> </div> },
//             2 => html! { <div> </div> },
//             _ => html! { <div> </div> },
//         };

//         if cx.cx.blah {
//             html! { <div> </div> }
//         } else {
//             tex
//         }
//     })
// }

// /// An example of a datafetching service
// async fn fetch_data() -> Result<String, ()> {
//     todo!()
// }
