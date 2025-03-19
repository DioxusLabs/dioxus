use dioxus::desktop::window;
use dioxus::prelude::*;

pub fn launch() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let value = 12312;

    rsx! {
        h2 { "iOS Binary Patching - {count}" }
        button { onclick: move |_| { count.set(count() + 1) }, "Increment {value}" }
        button { onclick: move |_| window().set_zoom_level(1.0), "Zoom 1x" }
        button { onclick: move |_| window().set_zoom_level(1.5), "Zoom 2x" }
        button { onclick: move |_| window().set_zoom_level(3.0), "Zoom 3x" }
        for x in 0..2 {
            Child { id: x, opt: "List entry" }
        }
    }
}

#[component]
fn Child(id: u32, opt: String) -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        div {
            h3 { "Child: {id} - {opt}" }
            p { "count: {count}" }
            button {
                onclick: move |_| {
                    count += id;
                },
                "Increment Count"
            }
        }
    }
}
#[component]
fn Child2(id: u32, opt: String) -> Element {
    rsx! {
        div { "oh lordy!" }
        div { "Hello ?? child2s: {id} - {opt} ?" }
    }
}

#[component]
fn Child3(id: u32, opt: String) -> Element {
    rsx! {
        div { "Hello ?? child: {id} - {opt} ?" }
    }
}

#[component]
fn Child4(id: u32, opt: String) -> Element {
    rsx! {
        div { "Hello ?? child: {id} - {opt} ?" }
        div { "Hello ?? child: {id} - {opt} ?" }
        div { "Hello ?? child: {id} - {opt} ?" }
    }
}
