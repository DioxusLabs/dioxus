use dioxus::prelude::*;
use dioxus::ssr;

fn main() {
    let mut vdom = VirtualDom::new(APP);
    let _ = vdom.rebuild();
    println!("{}", ssr::render_vdom(&vdom));
}

static APP: Component<()> = |cx| {
    cx.render(rsx!(
        div {
            h1 { "Title" }
            p { "Body" }
        }
    ))
};
