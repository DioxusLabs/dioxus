use dioxus::prelude::*;
use dioxus_html::input_data::keyboard_types::Code;

fn main() {
    dioxus_tui::launch(app);
}

#[component]
fn Button(color_offset: u32, layer: u16) -> Element {
    let mut toggle = use_signal(|| false);
    let mut hovered = use_signal(|| false);

    let hue = color_offset % 255;
    let saturation = if toggle() { 50 } else { 25 } + if hovered() { 50 } else { 25 };
    let brightness = saturation / 2;
    let color = format!("hsl({hue}, {saturation}, {brightness})");

    rsx! {
        div{
            width: "100%",
            height: "100%",
            background_color: "{color}",
            tabindex: "{layer}",
            onkeydown: move |e| {
                if let Code::Space = e.code() {
                    toggle.toggle();
                }
            },
            onclick: move |_| toggle.toggle(),
            onmouseenter: move |_| hovered.set(true),
            onmouseleave: move |_| hovered.set(false),
            justify_content: "center",
            align_items: "center",
            display: "flex",
            flex_direction: "column",
            p { "tabindex: {layer}" }
        }
    }
}

fn app() -> Element {
    rsx! {
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
    }
}
