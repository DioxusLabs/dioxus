#![allow(unused, non_upper_case_globals)]

use dioxus_core::prelude::*;

fn main() {}

static Example: FC<()> = |cx| {
    let (name, set_name) = use_state(&cx, || "...?");

    cx.render(rsx!(
        div {
            h1 { "Hello, {name}" }
            // look ma - we can rsx! and html! together
            {["jack", "jill"].iter().map(|f| html!(<button onclick={move |_| set_name(f)}> "{f}" </button>))}
        }
    ))
};

pub fn render<'src, 'a, F: for<'b> FnOnce(&'b NodeFactory<'src>) -> VNode<'src> + 'src + 'a, P>(
    cx: &'a Context<'src, P>,
    lazy_nodes: LazyNodes<'src, F>,
) -> VNode<'src> {
    cx.render(lazy_nodes)
}
