use std::{borrow::Borrow, marker::PhantomData, ops::Deref};

use builder::{button, div};
use dioxus_core::prelude::*;

fn main() {}
struct SomeContext {
    items: Vec<String>,
}

struct Props {
    name: String,
}

#[allow(unused)]
static Example: FC<Props> = |ctx, props| {
    let value = ctx.use_context(|c: &SomeContext| c.items.last().unwrap());

    ctx.view(move |bump| {
        button(bump)
            .on("click", move |_| {
                // //
                println!("Value is {}", props.name);
                println!("Value is {}", value.as_str());
                println!("Value is {}", *value);
            })
            //
            .on("click", move |_| {
                println!("Value is {}", props.name);
            })
            .finish()
    })
    // ctx.view(html! {
    //     <div>
    //         <button onclick={move |_| println!("Value is {}", value)} />
    //         <button onclick={move |_| println!("Value is {}", value)} />
    //         <button onclick={move |_| println!("Value is {}", value)} />
    //         <div>
    //             <p> "Value is: {val}" </p>
    //         </div>
    //     </div>
    // })
};
