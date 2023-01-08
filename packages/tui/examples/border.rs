use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch(app);
}

fn app(cx: Scope) -> Element {
    let radius = use_state(cx, || 0);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "100%",
            justify_content: "center",
            align_items: "center",
            background_color: "hsl(248, 53%, 58%)",
            onwheel: move |w| radius.modify(|r| (r + w.delta().strip_units().y as i8).abs()),

            border_style: "solid none solid double",
            border_width: "thick",
            border_radius: "{radius}px",
            border_color: "#0000FF #FF00FF #FF0000 #00FF00",

            "{radius}"
        }
    })
}
