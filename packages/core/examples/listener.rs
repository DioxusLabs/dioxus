#![allow(unused, non_upper_case_globals)]

use dioxus_core::prelude::*;

fn main() {
    Some(10)
        .map(|f| f * 5)
        .map(|f| f / 3)
        .map(|f| f * 5)
        .map(|f| f / 3);
}

static Example: FC<()> = |ctx, props| {
    let (name, set_name) = use_state(&ctx, || "...?");

    ctx.render(html! {
        <div>
            <h1> "Hello, {name}" </h1>
            <button onclick={move |_| set_name("jack")}> "jack!" </button>
            <button onclick={move |_| set_name("jill")}> "jill!" </button>
        </div>
    })
};
