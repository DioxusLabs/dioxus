use dioxus::prelude::*;

fn main() {
    dioxus_tui::launch_cfg(
        app,
        dioxus_tui::Config::default().with_rendering_mode(dioxus_tui::RenderingMode::Ansi),
    );
}

fn app() -> Element {
    let steps = 50;
    rsx! {
        div{
            width: "100%",
            height: "100%",
            flex_direction: "column",
            for x in 0..=steps {
                div { width: "100%", height: "100%", flex_direction: "row",
                    for y in 0..=steps {
                        {
                            let hue = x as f32*360.0/steps as f32;
                            let alpha = y as f32*100.0/steps as f32;
                            rsx! {
                                div {
                                    left: "{x}px",
                                    top: "{y}px",
                                    width: "10%",
                                    height: "100%",
                                    background_color: "hsl({hue}, 100%, 50%, {alpha}%)",
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
