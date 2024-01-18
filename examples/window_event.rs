use dioxus::prelude::*;
use dioxus_desktop::{window, Config, WindowBuilder};

fn main() {
    LaunchBuilder::desktop()
        .cfg(
            Config::new().with_window(
                WindowBuilder::new()
                    .with_title("Borderless Window")
                    .with_decorations(false),
            ),
        )
        .launch(app)
}

fn app() -> Element {
    let mut fullscreen = use_signal(|| false);
    let mut always_on_top = use_signal(|| false);
    let mut decorations = use_signal(|| false);

    rsx!(
        link {
            href: "https://unpkg.com/tailwindcss@^2/dist/tailwind.min.css",
            rel: "stylesheet"
        }
        header {
            class: "text-gray-400 bg-gray-900 body-font",
            onmousedown: move |_| window().drag(),
            div { class: "container mx-auto flex flex-wrap p-5 flex-col md:flex-row items-center",
                a { class: "flex title-font font-medium items-center text-white mb-4 md:mb-0",
                    span { class: "ml-3 text-xl", "Dioxus" }
                }
                nav { class: "md:ml-auto flex flex-wrap items-center text-base justify-center" }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.stop_propagation(),
                    onclick: move |_| window().set_minimized(true),
                    "Minimize"
                }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.stop_propagation(),
                    onclick: move |_| {
                        window().set_fullscreen(!fullscreen());
                        window().set_resizable(fullscreen());
                        fullscreen.toggle();
                    },
                    "Fullscreen"
                }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.stop_propagation(),
                    onclick: move |_| window().close(),
                    "Close"
                }
            }
        }
        br {}
        div { class: "container mx-auto",
            div { class: "grid grid-cols-5",
                div {
                    button {
                        class: "inline-flex items-center text-white bg-green-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
                        onmousedown: |evt| evt.stop_propagation(),
                        onclick: move |_| {
                            window().set_always_on_top(!always_on_top());
                            always_on_top.toggle();
                        },
                        "Always On Top"
                    }
                }
                div {
                    button {
                        class: "inline-flex items-center text-white bg-blue-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
                        onmousedown: |evt| evt.stop_propagation(),
                        onclick: move |_| {
                            window().set_decorations(!decorations());
                            decorations.toggle();
                        },
                        "Set Decorations"
                    }
                }
                div {
                    button {
                        class: "inline-flex items-center text-white bg-blue-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
                        onmousedown: |evt| evt.stop_propagation(),
                        onclick: move |_| window().set_title("Dioxus Application"),
                        "Change Title"
                    }
                }
            }
        }
    )
}
