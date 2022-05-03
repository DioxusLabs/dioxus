use dioxus::{events::KeyCode, prelude::*};

fn main() {
    dioxus::tui::launch(app);
}

#[derive(PartialEq, Props)]
struct ButtonProps {
    color_offset: u32,
    layer: u16,
}

#[allow(non_snake_case)]
fn Button(cx: Scope<ButtonProps>) -> Element {
    let toggle = use_state(&cx, || false);
    let hovered = use_state(&cx, || false);

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
            onkeydown: |e| {
                if let KeyCode::Space = e.data.key_code{
                    toggle.modify(|f| !f);
                }
            },
            onclick: |_| {
                toggle.modify(|f| !f);
            },
            onmouseenter: |_|{
                hovered.set(true);
            },
            onmouseleave: |_|{
                hovered.set(false);
            },
            justify_content: "center",
            align_items: "center",
            display: "flex",
            flex_direction: "column",

            p{"tabindex: {cx.props.layer}"}
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

            (1..8).map(|y|
                cx.render(rsx!{
                    div{
                        display: "flex",
                        flex_direction: "row",
                        width: "100%",
                        height: "100%",
                        (1..8).map(|x|{
                            if (x + y) % 2 == 0{
                                cx.render(rsx!{
                                    div{
                                        width: "100%",
                                        height: "100%",
                                        background_color: "rgb(100, 100, 100)",
                                    }
                                })
                            }
                            else{
                                let layer = (x + y) % 3;
                                cx.render(rsx!{
                                    Button{
                                        color_offset: x * y,
                                        layer: layer as u16,
                                    }
                                })
                            }
                        })
                    }
                })
            )
        }
    })
}
