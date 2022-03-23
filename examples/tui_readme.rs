use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let alpha = use_state(&cx, || 100);

    cx.render(rsx! {
        div {
            onwheel: move |evt| alpha.set((**alpha + evt.data.delta_y as i64).min(100).max(0)),

            width: "100%",
            height: "10px",
            background_color: "red",
            // justify_content: "center",
            // align_items: "center",

            p{
                color: "rgba(0, 255, 0, {alpha}%)",
                "Hello world!"
            }
            p{
                "{alpha}"
            }
            // p{"Hi"}
        }
    })
}
