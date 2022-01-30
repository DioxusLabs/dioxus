use dioxus::prelude::*;
use dioxus_desktop::desktop_context::DesktopContext;

fn main() {
    dioxus::desktop::launch_cfg(app, |cfg| {
        cfg.with_window(|w| {
            w.with_title("BorderLess Demo")
            .with_decorations(false)
        })
    });
}

fn app (cx: Scope) -> Element {
    let desktop = cx.consume_context::<DesktopContext>().unwrap();
    cx.render(rsx!(
        div {
            style: "background-color: black; height: 20px; width: 100%",
            onmousedown: move |_| desktop.drag_window(),
        }
    ))
}