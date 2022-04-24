use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch_cfg(app, |cfg| {
        cfg.with_window(|w| w.with_title("BorderLess Demo").with_decorations(false))
    });
}

fn app(cx: Scope) -> Element {
    let window = dioxus::desktop::use_window(&cx);

    // if you want to make window fullscreen, you need close the resizable.
    // window.set_fullscreen(true);
    // window.set_resizable(false);

    let fullscreen = use_state(&cx, || false);
    let always_on_top = use_state(&cx, || false);
    let decorations = use_state(&cx, || false);

    cx.render(rsx!(
        link { href:"https://unpkg.com/tailwindcss@^2/dist/tailwind.min.css", rel:"stylesheet" }
        header {
            class: "text-gray-400 bg-gray-900 body-font",
            onmousedown: move |_| window.drag(),
            div {
                class: "container mx-auto flex flex-wrap p-5 flex-col md:flex-row items-center",
                a { class: "flex title-font font-medium items-center text-white mb-4 md:mb-0",
                    span { class: "ml-3 text-xl", "Dioxus"}
                }
                nav { class: "md:ml-auto flex flex-wrap items-center text-base justify-center" }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.cancel_bubble(),
                    onclick: move |_| window.set_minimized(true),
                    "Minimize"
                }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.cancel_bubble(),
                    onclick: move |_| {

                        window.set_fullscreen(!**fullscreen);
                        window.set_resizable(**fullscreen);
                        fullscreen.modify(|f| !*f);
                    },
                    "Fullscreen"
                }
                button {
                    class: "inline-flex items-center bg-gray-800 border-0 py-1 px-3 focus:outline-none hover:bg-gray-700 rounded text-base mt-4 md:mt-0",
                    onmousedown: |evt| evt.cancel_bubble(),
                    onclick: move |_| window.close(),
                    "Close"
                }
            }
        }
        br {}
        div {
            class: "container mx-auto",
            div {
                class: "grid grid-cols-5",
                div {
                    button {
                        class: "inline-flex items-center text-white bg-green-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
                        onmousedown: |evt| evt.cancel_bubble(),
                        onclick: move |_| {
                            window.set_always_on_top(!always_on_top);
                            always_on_top.set(!always_on_top);
                        },
                        "Always On Top"
                    }
                }
                div {
                    button {
                        class: "inline-flex items-center text-white bg-blue-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
                        onmousedown: |evt| evt.cancel_bubble(),
                        onclick: move |_| {
                            window.set_decorations(!decorations);
                            decorations.set(!decorations);
                        },
                        "Set Decorations"
                    }
                }
                div {
                    button {
                        class: "inline-flex items-center text-white bg-blue-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
                        onmousedown: |evt| evt.cancel_bubble(),
                        onclick: move |_| {
                            window.set_title("Dioxus Application");
                        },
                        "Change Title"
                    }
                }
            }
        }
    ))
}
