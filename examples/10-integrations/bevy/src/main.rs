use std::any::Any;

use color::{palette::css::WHITE, parse_color};
use color::{OpaqueColor, Srgb};
use demo_renderer::{DemoMessage, DemoPaintSource};
use dioxus_native::prelude::*;
use dioxus_native::use_wgpu;
use wgpu::Limits;

mod bevy_renderer;
mod bevy_scene_plugin;
mod demo_renderer;

// CSS Styles
static STYLES: Asset = asset!("/src/styles.css");

type Color = OpaqueColor<Srgb>;

fn limits() -> Limits {
    Limits {
        max_storage_buffers_per_shader_stage: 12,
        ..Limits::default()
    }
}

fn main() {
    #[cfg(feature = "tracing")]
    tracing_subscriber::fmt::init();

    let config: Vec<Box<dyn Any>> = vec![Box::new(limits())];
    dioxus_native::launch_cfg(app, Vec::new(), config);
}

fn app() -> Element {
    let mut show_cube = use_signal(|| true);

    let color_str = use_signal(|| String::from("red"));
    let color = use_memo(move || {
        parse_color(&color_str())
            .map(|c| c.to_alpha_color())
            .unwrap_or(WHITE)
            .split()
            .0
    });

    use_effect(move || println!("{:?}", color().components));

    rsx!(
        Stylesheet { href: STYLES }
        div { id:"overlay",
            h2 { "Control Panel" },
            button {
                onclick: move |_| show_cube.toggle(),
                if show_cube() {
                    "Hide cube"
                } else {
                    "Show cube"
                }
            }
            br {}
            ColorControl { label: "Color:", color_str },
            p { "This overlay demonstrates that the custom Bevy content can be rendered beneath layers of HTML content" }
        }
        div { id:"underlay",
            h2 { "Underlay" },
            p { "This underlay demonstrates that the custom Bevy content can be rendered above layers and blended with the content underneath" }
        }
        header {
            h1 { "Blitz Bevy Demo" }
        }
        if show_cube() {
            SpinningCube { color }
        }
    )
}

#[component]
fn ColorControl(label: &'static str, color_str: WriteSignal<String>) -> Element {
    rsx!(div {
        class: "color-control",
        { label },
        input {
            value: color_str(),
            oninput: move |evt| {
                *color_str.write() = evt.value()
            }
        }
    })
}

#[component]
fn SpinningCube(color: Memo<Color>) -> Element {
    // Create custom paint source and register it with the renderer
    let paint_source = DemoPaintSource::new();
    let sender = paint_source.sender();
    let paint_source_id = use_wgpu(move || paint_source);

    use_effect(move || {
        sender.send(DemoMessage::SetColor(color())).unwrap();
    });

    rsx!(
        div { id:"canvas-container",
            canvas {
                id: "demo-canvas",
                "src": paint_source_id
            }
        }
    )
}
