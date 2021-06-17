#![allow(unused, non_upper_case_globals)]

use dioxus_core::prelude::*;

fn main() {
    Some(10)
        .map(|f| f * 5)
        .map(|f| f / 3)
        .map(|f| f * 5)
        .map(|f| f / 3);
}

static Example: FC<()> = |ctx| {
    let (name, set_name) = use_state(&ctx, || "...?");

    ctx.render(rsx!(
        div {
            h1 { "Hello, {name}" }
            // look ma - we can rsx! and html! together
            {["jack", "jill"].iter().map(|f| html!(<button onclick={move |_| set_name(f)}> "{f}" </button>))}
        }
    ))
};

pub fn render<'src, 'a, F: for<'b> FnOnce(&'b NodeCtx<'src>) -> VNode<'src> + 'src + 'a, P>(
    ctx: &'a Context<'src, P>,
    lazy_nodes: LazyNodes<'src, F>,
) -> VNode<'src> {
    ctx.render(lazy_nodes)
}
