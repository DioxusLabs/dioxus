use dioxus::prelude::*;
use dioxus_html::input_data::keyboard_types::Code;

fn main() {
    dioxus_tui::launch(app);
}

#[derive(PartialEq, Props)]
struct ButtonProps {
    color_offset: u32,
    layer: u16,
}

#[allow(non_snake_case)]
fn Button(cx: Scope<ButtonProps>) -> Element {
    let toggle = use_state(cx, || false);
    let hovered = use_state(cx, || false);

    let hue = cx.props.color_offset % 255;
    let saturation = if *toggle.get() { 50 } else { 25 } + if *hovered.get() { 50 } else { 25 };
    let brightness = saturation / 2;
    let color = format!("hsl({hue}, {saturation}, {brightness})");

    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            background_color: "{color}",
            tabindex: "{cx.props.layer}",
            onkeydown: move |e| {
                if let Code::Space = e.inner().code() {
                    toggle.modify(|f| !f);
                }
            },
            onclick: move |_| {
                toggle.modify(|f| !f);
            },
            onmouseenter: move |_| {
                hovered.set(true);
            },
            onmouseleave: move |_|{
                hovered.set(false);
            },
            justify_content: "center",
            align_items: "center",
            display: "flex",
            flex_direction: "column",
            p{ "tabindex: {cx.props.layer}" }
        }
    })
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            width: "100%",
            height: "100%",
            for y in 1..8 {
                div {
                    display: "flex",
                    flex_direction: "row",
                    width: "100%",
                    height: "100%",
                    for x in 1..8 {
                        if (x + y) % 2 == 0 {
                            div {
                                width: "100%",
                                height: "100%",
                                background_color: "rgb(100, 100, 100)",
                            }
                        } else {
                            Button {
                                color_offset: x * y,
                                layer: ((x + y) % 3) as u16,
                            }
                        }
                    }
                }
            }
        }
    })
}
