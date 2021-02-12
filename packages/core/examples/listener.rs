#![allow(unused, non_upper_case_globals)]

use dioxus_core::prelude::*;

fn main() {}

/*
Our flagship demo :)

*/
static Example: FC<()> = |ctx, props| {
    let (val1, set_val1) = use_state(&ctx, || "b1");

    ctx.view(html! {
        <div>
            <button onclick={move |_| set_val1("b1")}> "Set value to b1" </button>
            <button onclick={move |_| set_val1("b2")}> "Set value to b2" </button>
            <button onclick={move |_| set_val1("b3")}> "Set value to b3" </button>
            <div>
                <p> "Value is: {val1}" </p>
            </div>
        </div>
    })
};
