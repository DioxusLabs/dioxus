use dioxus::prelude::*;

fn main() {
    dioxus::tui::launch_cfg(
        app,
        dioxus::tui::Config {
            rendering_mode: dioxus::tui::RenderingMode::Ansi,
            ..Default::default()
        },
    );
}

fn app(cx: Scope) -> Element {
    let steps = 50;
    cx.render(rsx! {
        div{
            width: "100%",
            height: "100%",
            flex_direction: "column",
            (0..=steps).map(|x|
                {
                    let hue = x as f32*360.0/steps as f32;
                    cx.render(rsx! {
                        div{
                            width: "100%",
                            height: "100%",
                            flex_direction: "row",
                            (0..=steps).map(|y|
                                {
                                    let alpha = y as f32*100.0/steps as f32;
                                    cx.render(rsx! {
                                        div {
                                            left: "{x}px",
                                            top: "{y}px",
                                            width: "10%",
                                            height: "100%",
                                            background_color: "hsl({hue}, 100%, 50%, {alpha}%)",
                                        }
                                    })
                                }
                            )
                        }
                    })
                }
            )
        }
    })
}
