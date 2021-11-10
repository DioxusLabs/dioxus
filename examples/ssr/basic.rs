use dioxus::virtual_dom::VirtualDom;
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
fn main() {
    let mut dom = VirtualDom::new(App);
    dom.rebuild();
    println!(
        "{}",
        dioxus_ssr::render_vdom(&dom, |c| c.newline(true).indent(true))
    )
}

pub static App: FC<()> = |cx, props| {
    cx.render(rsx!(
        div {
            class: "overflow-hidden"
            ul {
                {(0..10).map(|i| rsx!{ li { class: "flex flex-col", "entry: {i}"}})}
            }
            "hello world!"
        }
    ))
};
