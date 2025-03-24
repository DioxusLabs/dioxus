use dioxus::prelude::*;

pub fn launch() {
    dioxus::launch(app);
}

fn app() -> Element {
    let count = 123456;

    rsx! {
        h1 { "Dioxus Hot-patch: {count}" }
        div { style: "display: flex; flex-row: column; align-items: center; justify-content: center;",
            img { src: "https://rustacean.net/assets/rustacean-flat-happy.png" }
            div {
                for x in 0..3 {
                    Child { id: x + 1, opt: "List entry" }
                }
            }
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

#[component]
fn ZoomComponent() -> Element {
    // use dioxus::desktop::window;
    // button { onclick: move |_| window().set_zoom_level(1.0), "Zoom 1x" }
    // button { onclick: move |_| window().set_zoom_level(1.5), "Zoom 1.5x" }
    // button { onclick: move |_| window().set_zoom_level(2.0), "Zoom 2x" }
    // button { onclick: move |_| window().set_zoom_level(3.0), "Zoom 3x" }
    rsx! {
        div { "Zoom me!" }
    }
}
