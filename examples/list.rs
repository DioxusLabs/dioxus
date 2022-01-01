use dioxus::prelude::*;

fn main() {
    let mut dom = VirtualDom::new(app);
    dom.rebuild();

    rink::render_vdom(&mut dom).unwrap();
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            flex_direction: "column",
            border_width: "1px",

            h1 { height: "2px", color: "green",
                "that's awesome!"
            }

            ul {
                flex_direction: "column",
                padding_left: "3px",
                (0..10).map(|i| rsx!(
                    "> hello {i}"
                ))
            }
        }
    })
}
