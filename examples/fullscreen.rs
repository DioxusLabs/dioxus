use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch_cfg(app, |cfg| {
        cfg.with_window(|w| w.with_title("BorderLess Demo").with_decorations(false))
    });
}

fn app(cx: Scope) -> Element {
    let window = dioxus::desktop::use_window(&cx);

    // window.set_fullscreen(true);

    cx.render(rsx!(
        link { href:"https://unpkg.com/tailwindcss@^2/dist/tailwind.min.css", rel:"stylesheet" }
        header {
            class: "text-gray-400 bg-gray-900 body-font",
            onmousedown: move |_| window.drag(),
            div {
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.cancel_bubble(),
                    onclick: move |_| window.set_fullscreen(true),
                    "Enter FullScreen"
                }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.cancel_bubble(),
                    onclick: move |_| window.set_fullscreen(false),
                    "Exit FullScreen"
                }
            }
        }
    ))
}
