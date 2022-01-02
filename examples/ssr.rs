use dioxus::prelude::*;
use dioxus::ssr;

fn main() {
    let mut vdom = VirtualDom::new(app);
    let _ = vdom.rebuild();
    println!("{}", ssr::render_vdom(&vdom));
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            h1 { "Title" }
            p { "Body" }
        }
    ))
}
