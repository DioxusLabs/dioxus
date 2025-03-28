use dioxus::prelude::*;

pub fn launch() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center;",
            h1 { "Apple: {count} ???" }
            button { onclick: move |_| count += 1, "Incr" }
            button { onclick: move |_| count -= 1, "Decr" }
            img {  width: "300px", src: "https://rustacean.net/assets/rustacean-flat-happy.png" }
        }
        div { style: "display: flex; flex-direction: column; align-items: center; justify-content: center;",
            div { style: "background-color: red",
                for x in 0..1 {
                    Child { id: x + 1, opt: "List entry", color: "gris" }
                }
            }
            div { style: "background-color: orange",
                for x in 0..1 {
                    Child { id: x + 1, opt: "List entry", color: "blue" }
                }
            }
            div { style: "background-color: yellow",
                for x in 0..1 {
                    Child { id: x + 1, opt: "List entry", color: "yellow" }
                }
            }
            div { style: "background-color: green",
                for x in 0..1 {
                    Child { id: x + 10, opt: "List entry", color: "orange" }
                }
            }
            div { style: "background-color: blue",
                for x in 0..1 {
                    Child { id: x + 10, opt: "List entry", color: "bluebleu" }
                }
            }
            div { style: "background-color: indigo",
                for x in 0..1 {
                    Child { id: x + 10, opt: "List entry", color: "bluebleu" }
                }
            }
        }
    }
}

#[component]
fn Child(id: u32, opt: String, color: String) -> Element {
    let mut count = use_signal(|| 0);

    rsx! {
        div {
            h3 { "Chil!!!!!!!!!! {id} - {opt} - {color} - {color} - {color}" }
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
