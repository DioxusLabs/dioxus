use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    rink::render_vdom(&dom).unwrap();
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",

            "Hello world!"
        }
    })
}
